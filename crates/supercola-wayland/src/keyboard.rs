//! Injeta Ctrl+V (ou Ctrl+Shift+V) pelo teclado virtual do Wayland.
//!
//! Há duas sequências, porque os compositores tratam o mapa do teclado virtual de
//! formas incompatíveis:
//!
//! - **COSMIC:** o mapa virtual só é ativado na primeira tecla, e essa ativação recria
//!   o estado do teclado. Isso apaga o Ctrl enviado por `modifiers`, além de NumLock e
//!   CapsLock. Com `no_keymap` as teclas usam o mapa atual, e nada disso acontece.
//! - **Os demais (wlroots):** `no_keymap` não existe, e o modificador só vale por
//!   `modifiers`. O mapa é `us`, para o `V` cair na tecla certa em qualquer layout.
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::fs::FileExt;
use std::time::Duration;

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_keyboard::{self, KeymapFormat, WlKeyboard};
use wayland_client::protocol::{wl_registry::WlRegistry, wl_seat::WlSeat};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1 as Manager,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1 as Keyboard,
};
use xkbcommon::xkb;

const PRESSIONADA: u32 = 1;
const SOLTA: u32 = 0;
/// Códigos xkb ficam 8 acima dos códigos evdev usados pelo protocolo.
const EVDEV_PARA_XKB: u32 = 8;
const PAUSA: Duration = Duration::from_millis(20);
/// O app recebe o mapa novo antes da tecla: sem folga, a primeira tecla pode se perder.
const FOLGA_DO_MAPA: Duration = Duration::from_millis(60);

/// Códigos evdev das teclas da colagem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Teclas {
    ctrl: u32,
    shift: u32,
    v: u32,
}

impl Teclas {
    /// Posições físicas de um teclado comum, que valem no mapa `us`.
    const FISICAS: Self = Self {
        ctrl: 29,
        shift: 42,
        v: 47,
    };

    /// Sem a tecla no mapa, como em um layout só cirílico, fica a posição física.
    fn do_mapa(mapa: &xkb::Keymap) -> Self {
        Self {
            ctrl: codigo_de(mapa, xkb::keysyms::KEY_Control_L).unwrap_or(Self::FISICAS.ctrl),
            shift: codigo_de(mapa, xkb::keysyms::KEY_Shift_L).unwrap_or(Self::FISICAS.shift),
            v: codigo_de(mapa, xkb::keysyms::KEY_v).unwrap_or(Self::FISICAS.v),
        }
    }
}

/// Procura o símbolo no nível base de cada tecla, começando pelo primeiro layout.
fn codigo_de(mapa: &xkb::Keymap, simbolo: u32) -> Option<u32> {
    let (min, max) = (mapa.min_keycode().raw(), mapa.max_keycode().raw());
    (0..mapa.num_layouts()).find_map(|layout| {
        (min..=max)
            .find(|bruto| {
                mapa.key_get_syms_by_level(xkb::Keycode::new(*bruto), layout, 0)
                    .iter()
                    .any(|s| s.raw() == simbolo)
            })
            .map(|bruto| bruto - EVDEV_PARA_XKB)
    })
}

#[derive(Default)]
struct State {
    mapa_do_seat: Option<(OwnedFd, u32)>,
}

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlKeyboard, ()> for State {
    fn event(
        state: &mut Self,
        _: &WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Keymap {
            format: WEnum::Value(KeymapFormat::XkbV1),
            fd,
            size,
        } = event
        {
            state.mapa_do_seat = Some((fd, size));
        }
    }
}

delegate_noop!(State: ignore WlSeat);
delegate_noop!(State: Manager);
delegate_noop!(State: Keyboard);

struct Sessao {
    conn: Connection,
    queue: EventQueue<State>,
    state: State,
    seat: WlSeat,
    teclado: Keyboard,
    cosmic: bool,
    tempo: u32,
}

impl Sessao {
    fn abrir() -> crate::R<Self> {
        let conn = Connection::connect_to_env()?;
        let (globals, queue) = registry_queue_init::<State>(&conn)?;
        let qh = queue.handle();
        let manager: Manager = globals.bind(&qh, 1..=1, ())?;
        let seat: WlSeat = globals.bind(&qh, 1..=1, ())?;
        let teclado = manager.create_virtual_keyboard(&seat, &qh, ());
        let cosmic = globals
            .contents()
            .with_list(|lista| lista.iter().any(|g| g.interface.starts_with("zcosmic_")));
        Ok(Self {
            conn,
            queue,
            state: State::default(),
            seat,
            teclado,
            cosmic,
            tempo: 0,
        })
    }

    fn sincronizar(&mut self) -> crate::R {
        self.queue.roundtrip(&mut self.state)?;
        Ok(())
    }

    fn tecla(&mut self, codigo: u32, estado: u32) -> crate::R {
        self.teclado.key(self.tempo, codigo, estado);
        self.tempo += PAUSA.as_millis() as u32;
        self.sincronizar()?;
        std::thread::sleep(PAUSA);
        Ok(())
    }

    fn modificadores(&mut self, mascara: u32) -> crate::R {
        self.teclado.modifiers(mascara, 0, 0, 0);
        self.sincronizar()?;
        std::thread::sleep(PAUSA);
        Ok(())
    }

    fn ler_mapa_do_seat(&mut self) -> crate::R<xkb::Keymap> {
        self.seat.get_keyboard(&self.queue.handle(), ());
        self.sincronizar()?;
        let (fd, tamanho) = self
            .state
            .mapa_do_seat
            .take()
            .ok_or("o compositor não informou o mapa de teclas")?;
        let mut dados = vec![0; tamanho as usize];
        // `pread`: o descritor pode ser compartilhado com outros clientes.
        File::from(fd).read_exact_at(&mut dados, 0)?;
        let fim = dados.iter().position(|b| *b == 0).unwrap_or(dados.len());
        dados.truncate(fim);
        xkb::Keymap::new_from_string(
            &xkb::Context::new(xkb::CONTEXT_NO_FLAGS),
            String::from_utf8(dados)?,
            xkb::KEYMAP_FORMAT_TEXT_V1,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .ok_or_else(|| "não consegui ler o mapa de teclas".into())
    }

    fn usar_mapa_atual(&mut self) -> crate::R {
        let vazio = File::open("/dev/null")?;
        self.teclado
            .keymap(KeymapFormat::NoKeymap as u32, vazio.as_fd(), 0);
        self.sincronizar()
    }

    /// Prepara o `no_keymap`. Até aqui nenhuma tecla foi enviada.
    fn preparar_mapa_atual(&mut self) -> crate::R<Teclas> {
        let teclas = Teclas::do_mapa(&self.ler_mapa_do_seat()?);
        self.usar_mapa_atual()?;
        Ok(teclas)
    }

    fn colar_no_mapa_atual(&mut self, teclas: Teclas, shift: bool) -> crate::R {
        self.tecla(teclas.ctrl, PRESSIONADA)?;
        if shift {
            self.tecla(teclas.shift, PRESSIONADA)?;
        }
        self.tecla(teclas.v, PRESSIONADA)?;
        self.tecla(teclas.v, SOLTA)?;
        if shift {
            self.tecla(teclas.shift, SOLTA)?;
        }
        self.tecla(teclas.ctrl, SOLTA)
    }

    /// O Ctrl real ativa o mapa no COSMIC, e `modifiers` vale nos demais compositores.
    fn colar_com_mapa_us(&mut self, shift: bool) -> crate::R {
        let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let mapa =
            xkb::Keymap::new_from_names(&ctx, "", "", "us", "", None, xkb::KEYMAP_COMPILE_NO_FLAGS)
                .ok_or("não consegui compilar o keymap")?;
        let texto = mapa.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1);

        let caminho = supercola_core::xdg::runtime_dir()
            .join(format!("supercola-{}.xkb", std::process::id()));
        let mut arquivo = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&caminho)?;
        std::fs::remove_file(&caminho)?;
        arquivo.write_all(texto.as_bytes())?;
        arquivo.write_all(&[0])?;
        self.teclado.keymap(
            KeymapFormat::XkbV1 as u32,
            arquivo.as_fd(),
            texto.len() as u32 + 1,
        );
        self.sincronizar()?;

        let mut mascara = 1 << mapa.mod_get_index(xkb::MOD_NAME_CTRL);
        let teclas = Teclas::FISICAS;
        self.tecla(teclas.ctrl, PRESSIONADA)?;
        if shift {
            mascara |= 1 << mapa.mod_get_index(xkb::MOD_NAME_SHIFT);
            self.tecla(teclas.shift, PRESSIONADA)?;
        }
        self.modificadores(mascara)?;
        std::thread::sleep(FOLGA_DO_MAPA);
        self.tecla(teclas.v, PRESSIONADA)?;
        self.tecla(teclas.v, SOLTA)?;
        self.modificadores(0)?;
        if shift {
            self.tecla(teclas.shift, SOLTA)?;
        }
        self.tecla(teclas.ctrl, SOLTA)
    }

    fn encerrar(self) -> crate::R {
        self.teclado.destroy();
        self.conn.flush()?;
        Ok(())
    }
}

/// `SUPERCOLA_PASTE_MODE` força uma das sequências nos testes.
fn usa_mapa_atual(cosmic: bool) -> bool {
    match std::env::var("SUPERCOLA_PASTE_MODE").as_deref() {
        Ok("seat") => true,
        Ok("us") => false,
        _ => cosmic,
    }
}

pub fn paste(shift: bool) -> crate::R {
    let mut sessao = Sessao::abrir()?;
    if !usa_mapa_atual(sessao.cosmic) {
        sessao.colar_com_mapa_us(shift)?;
        return sessao.encerrar();
    }
    match sessao.preparar_mapa_atual() {
        Ok(teclas) => {
            sessao.colar_no_mapa_atual(teclas, shift)?;
            sessao.encerrar()
        }
        // Um erro de protocolo encerra a conexão: a reserva precisa de outra.
        Err(e) => {
            eprintln!("supercola: mapa atual indisponível, usando o mapa us: {e}");
            let mut reserva = Sessao::abrir()?;
            reserva.colar_com_mapa_us(shift)?;
            reserva.encerrar()
        }
    }
}

/// Aperta uma tecla pelo nome xkb, como `Return`. Só funciona no COSMIC, e existe para
/// os testes na sessão real: não troca o mapa de teclas, ao contrário do `wtype`.
pub fn tap(nome: &str) -> crate::R {
    let simbolo = xkb::keysym_from_name(nome, xkb::KEYSYM_NO_FLAGS).raw();
    let mut sessao = Sessao::abrir()?;
    let mapa = sessao.ler_mapa_do_seat()?;
    let codigo = codigo_de(&mapa, simbolo).ok_or_else(|| format!("sem a tecla {nome} no mapa"))?;
    sessao.usar_mapa_atual()?;
    sessao.tecla(codigo, PRESSIONADA)?;
    sessao.tecla(codigo, SOLTA)?;
    sessao.encerrar()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapa(layout: &str, variante: &str) -> xkb::Keymap {
        let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        xkb::Keymap::new_from_names(
            &ctx,
            "",
            "",
            layout,
            variante,
            None,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .expect("mapa de teclas")
    }

    #[test]
    fn no_mapa_us_as_teclas_ficam_nas_posicoes_fisicas() {
        assert_eq!(Teclas::do_mapa(&mapa("us", "")), Teclas::FISICAS);
    }

    #[test]
    fn no_abnt2_o_v_nao_muda_de_lugar() {
        assert_eq!(Teclas::do_mapa(&mapa("br", "")), Teclas::FISICAS);
    }

    #[test]
    fn no_dvorak_o_v_muda_de_lugar_e_o_ctrl_nao() {
        let t = Teclas::do_mapa(&mapa("us", "dvorak"));
        assert_ne!(t.v, Teclas::FISICAS.v);
        assert_eq!(t.ctrl, Teclas::FISICAS.ctrl);
    }

    #[test]
    fn layout_sem_v_usa_a_posicao_fisica() {
        assert_eq!(Teclas::do_mapa(&mapa("ru", "")).v, Teclas::FISICAS.v);
    }

    #[test]
    fn com_dois_layouts_o_primeiro_decide() {
        let dvorak_primeiro = mapa("us,us", "dvorak,");
        assert_ne!(Teclas::do_mapa(&dvorak_primeiro).v, Teclas::FISICAS.v);
    }
}
