// Liga o daemon do Super Cola ao GNOME Shell, que não oferece aos apps os protocolos usados
// nos outros ambientes: posiciona o painel, informa o foco e os formatos copiados, entrega
// o que o daemon pedir e aperta Ctrl+V.
//
// O conteúdo só sai por resposta a uma chamada do daemon, e nunca por sinal, que todo o
// barramento recebe. Quem decide o que pode ser lido é o daemon, a partir dos formatos.
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const APP_ID = 'dev.supercola.SuperCola';
const TITULO_PAINEL = 'Super Cola';
const DAEMON = 'dev.supercola.Daemon';
const DAEMON_CAMINHO = '/dev/supercola/Bridge';
const DAEMON_INTERFACE = 'dev.supercola.Bridge';
const CAMINHO = '/dev/supercola/Shell';
const ACESSO_NEGADO = 'org.freedesktop.DBus.Error.AccessDenied';
const FALHA = 'org.freedesktop.DBus.Error.Failed';
/// Um byte acima do limite do histórico, para o daemon saber que o item passou dele.
const LIMITE_BYTES = 4 * 1024 * 1024 + 1;
const PAUSA_TECLA_MS = 20;
const ASSENTAR_SELECAO_MS = 30;
const CLIPBOARD = Meta.SelectionType.SELECTION_CLIPBOARD;

const DESCRICAO = `<node>
  <interface name="dev.supercola.Shell">
    <method name="ReadSelection">
      <arg name="formato" type="s" direction="in"/>
      <arg name="dados" type="ay" direction="out"/>
    </method>
    <method name="SetSelection">
      <arg name="formato" type="s" direction="in"/>
      <arg name="dados" type="ay" direction="in"/>
    </method>
    <method name="Paste"><arg name="shift" type="b" direction="in"/></method>
    <method name="SetPosition"><arg name="posicao" type="s" direction="in"/></method>
  </interface>
</node>`;

function limitar(valor, minimo, maximo) {
    return Math.min(Math.max(valor, minimo), Math.max(minimo, maximo));
}

function idDoApp(janela) {
    return janela.get_gtk_application_id() ?? janela.get_wm_class() ?? '';
}

function ehPainel(janela) {
    return idDoApp(janela) === APP_ID && janela.get_title() === TITULO_PAINEL;
}

export default class SuperColaExtension extends Extension {
    enable() {
        this._posicao = 'bottom-right';
        this._dono = null;
        this._escritaPropria = false;
        this._avisoPendente = 0;
        this._pendentes = new Set();
        this._selecao = global.display.get_selection();

        const assento = Clutter.get_default_backend().get_default_seat();
        this._teclado = assento.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);

        this._dbus = Gio.DBusExportedObject.wrapJSObject(DESCRICAO, this);
        this._dbus.export(Gio.DBus.session, CAMINHO);

        this._vigia = Gio.bus_watch_name(Gio.BusType.SESSION, DAEMON,
            Gio.BusNameWatcherFlags.NONE,
            (_conexao, _nome, dono) => this._daemonChegou(dono),
            () => {
                this._dono = null;
            });

        this._selecaoId = this._selecao.connect('owner-changed',
            (_selecao, tipo) => this._selecaoMudou(tipo));
        this._focoId = global.display.connect('notify::focus-window', () => this._focoMudou());
        this._janelaId = global.display.connect('window-created',
            (_display, janela) => this._janelaCriada(janela));
    }

    disable() {
        this._pendentes.forEach(fonte => GLib.source_remove(fonte));
        this._pendentes.clear();
        this._cancelarAviso();
        global.display.disconnect(this._janelaId);
        global.display.disconnect(this._focoId);
        this._selecao.disconnect(this._selecaoId);
        Gio.bus_unwatch_name(this._vigia);
        this._dbus.unexport();
        this._dbus = null;
        this._teclado = null;
        this._selecao = null;
        this._dono = null;
    }

    _avisar(metodo, args) {
        if (!this._dono)
            return;
        Gio.DBus.session.call(DAEMON, DAEMON_CAMINHO, DAEMON_INTERFACE, metodo, args, null,
            Gio.DBusCallFlags.NONE, -1, null, null);
    }

    _daemonChegou(dono) {
        this._dono = dono;
        this._focoMudou();
        this._selecaoMudou(CLIPBOARD);
    }

    _selecaoMudou(tipo) {
        // O clipboard devolvido depois de um emoji não volta ao histórico.
        if (tipo !== CLIPBOARD || this._escritaPropria || this._avisoPendente)
            return;
        // Quem troca o próprio conteúdo solta a seleção antes de pegar a nova, e o Mutter
        // assume nesse meio-tempo. Avisar logo faria o daemon ler a seleção nova com as
        // regras da antiga: um emoji entraria no histórico.
        this._avisoPendente = GLib.timeout_add(GLib.PRIORITY_DEFAULT, ASSENTAR_SELECAO_MS, () => {
            this._avisoPendente = 0;
            const formatos = this._selecao.get_mimetypes(CLIPBOARD);
            if (formatos.length > 0)
                this._avisar('SelectionChanged', new GLib.Variant('(as)', [formatos]));
            return GLib.SOURCE_REMOVE;
        });
    }

    _cancelarAviso() {
        if (this._avisoPendente)
            GLib.source_remove(this._avisoPendente);
        this._avisoPendente = 0;
    }

    _focoMudou() {
        const janela = global.display.focus_window;
        if (janela && idDoApp(janela) !== APP_ID)
            this._avisar('FocusChanged', new GLib.Variant('(s)', [idDoApp(janela)]));
    }

    // Em Wayland, o app e o título só são conhecidos depois do primeiro quadro. O sinal vem
    // no meio do desenho, e mexer na janela nesse momento derruba o Shell inteiro.
    _janelaCriada(janela) {
        const ator = janela.get_compositor_private();
        if (!ator)
            return;
        const id = ator.connect('first-frame', () => {
            ator.disconnect(id);
            this._depoisDoQuadro(janela);
        });
    }

    _depoisDoQuadro(janela) {
        const fonte = GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
            this._pendentes.delete(fonte);
            if (janela.get_compositor_private() && ehPainel(janela))
                this._mostrarPainel(janela);
            return GLib.SOURCE_REMOVE;
        });
        this._pendentes.add(fonte);
    }

    _mostrarPainel(janela) {
        const monitor = global.display.get_current_monitor();
        const area = global.workspace_manager.get_active_workspace()
            .get_work_area_for_monitor(monitor);
        const quadro = janela.get_frame_rect();
        const direita = area.x + area.width - quadro.width;
        const baixo = area.y + area.height - quadro.height;
        let [x, y] = [direita, baixo];
        if (this._posicao === 'near-pointer') {
            const [px, py] = global.get_pointer();
            x = limitar(px, area.x, direita);
            y = limitar(py - quadro.height, area.y, baixo);
        }
        janela.move_frame(true, x, y);
        janela.make_above();
        janela.stick();
        Main.activateWindow(janela);
    }

    _autorizado(chamada) {
        if (this._dono && chamada.get_sender() === this._dono)
            return true;
        chamada.return_dbus_error(ACESSO_NEGADO, 'só o daemon do Super Cola');
        return false;
    }

    ReadSelectionAsync([formato], chamada) {
        if (!this._autorizado(chamada))
            return;
        const saida = Gio.MemoryOutputStream.new_resizable();
        this._selecao.transfer_async(CLIPBOARD, formato, LIMITE_BYTES, saida, null,
            (selecao, resultado) => {
                try {
                    selecao.transfer_finish(resultado);
                    saida.close(null);
                    const dados = GLib.Variant.new_from_bytes(new GLib.VariantType('ay'),
                        saida.steal_as_bytes(), true);
                    chamada.return_value(GLib.Variant.new_tuple([dados]));
                } catch (e) {
                    chamada.return_dbus_error(FALHA, e.message);
                }
            });
    }

    SetSelectionAsync([formato, dados], chamada) {
        if (!this._autorizado(chamada))
            return;
        // O aviso de troca de dono vem durante a gravação. Se a marca sobrasse, engoliria a
        // próxima cópia do usuário.
        this._cancelarAviso();
        this._escritaPropria = true;
        St.Clipboard.get_default().set_content(St.ClipboardType.CLIPBOARD, formato,
            new GLib.Bytes(dados));
        this._escritaPropria = false;
        chamada.return_value(null);
    }

    SetPositionAsync([posicao], chamada) {
        if (!this._autorizado(chamada))
            return;
        this._posicao = posicao;
        chamada.return_value(null);
    }

    PasteAsync([shift], chamada) {
        if (!this._autorizado(chamada))
            return;
        const modificadores = shift
            ? [Clutter.KEY_Control_L, Clutter.KEY_Shift_L]
            : [Clutter.KEY_Control_L];
        const passos = [
            ...modificadores.map(tecla => [tecla, Clutter.KeyState.PRESSED]),
            [Clutter.KEY_v, Clutter.KeyState.PRESSED],
            [Clutter.KEY_v, Clutter.KeyState.RELEASED],
            ...modificadores.reverse().map(tecla => [tecla, Clutter.KeyState.RELEASED]),
        ];
        GLib.timeout_add(GLib.PRIORITY_DEFAULT, PAUSA_TECLA_MS, () => {
            const passo = passos.shift();
            if (!passo || !this._teclado) {
                chamada.return_value(null);
                return GLib.SOURCE_REMOVE;
            }
            this._teclado.notify_keyval(Clutter.CURRENT_TIME, passo[0], passo[1]);
            return GLib.SOURCE_CONTINUE;
        });
    }
}
