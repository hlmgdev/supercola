//! Serviço D-Bus do daemon: recebe do ambiente gráfico o que o Wayland não informa.
//!
//! - **KDE:** um script carregado no KWin avisa qual app está em foco.
//! - **GNOME:** a extensão do Super Cola avisa o foco e quais formatos foram copiados, e
//!   o daemon pede só o que pode ler. Conteúdo não trafega em sinais, que todo o barramento
//!   recebe, e os dois lados só aceitam chamadas um do outro.
use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gio::{self, prelude::*, BusNameOwnerFlags, BusNameWatcherFlags, BusType, DBusCallFlags};
use gtk4::{glib, Application};
use supercola_core::config::Position;
use supercola_core::filter::{self, Decisao};
use supercola_core::history::MAX_ITEM_BYTES;
use supercola_core::model::{self, Flavor};
use supercola_wayland::clipboard::{Event, Handle, Oferta, Sink};
use supercola_wayland::focus::Focus;

const NOME: &str = "dev.supercola.Daemon";
const CAMINHO: &str = "/dev/supercola/Bridge";
const INTERFACE: &str = "dev.supercola.Bridge";
const DESCRICAO: &str = r#"<node>
  <interface name="dev.supercola.Bridge">
    <method name="FocusChanged"><arg name="app" type="s" direction="in"/></method>
    <method name="SelectionChanged"><arg name="formatos" type="as" direction="in"/></method>
  </interface>
</node>"#;

const KWIN: &str = "org.kde.KWin";
const KWIN_SCRIPTS: &str = "/Scripting";
const KWIN_INTERFACE: &str = "org.kde.kwin.Scripting";
const PLUGIN: &str = "supercola-foco";
/// Relativo à pasta de dados do Super Cola.
pub const SCRIPT_KWIN: &str = "kwin/focus.js";

const GNOME_SHELL: &str = "org.gnome.Shell";
pub const EXTENSAO: &str = "supercola@hlmgdev.github.io";
const SHELL_CAMINHO: &str = "/dev/supercola/Shell";
const SHELL_INTERFACE: &str = "dev.supercola.Shell";

const TEMPO_MS: i32 = 5000;

/// Dono atual do nome que pode falar com o daemon.
type Dono = Rc<RefCell<Option<String>>>;
type Conexao = Rc<RefCell<Option<gio::DBusConnection>>>;

/// Chamadas à extensão do GNOME.
#[derive(Clone)]
pub struct Shell {
    conexao: Conexao,
}

impl Shell {
    fn chamar(
        &self,
        metodo: &str,
        args: &glib::Variant,
        resposta: Option<&glib::VariantTy>,
    ) -> impl std::future::Future<Output = Result<glib::Variant, glib::Error>> + use<> {
        let conexao = self.conexao.borrow().clone();
        let (metodo, args) = (metodo.to_string(), args.clone());
        let resposta = resposta.map(ToOwned::to_owned);
        async move {
            let Some(conn) = conexao else {
                return Err(glib::Error::new(
                    gio::IOErrorEnum::NotConnected,
                    "sem o barramento de sessão",
                ));
            };
            conn.call_future(
                Some(GNOME_SHELL),
                SHELL_CAMINHO,
                SHELL_INTERFACE,
                &metodo,
                Some(&args),
                resposta.as_deref(),
                DBusCallFlags::NONE,
                TEMPO_MS,
            )
            .await
        }
    }

    pub async fn colar(&self, shift: bool) -> Result<(), glib::Error> {
        self.chamar("Paste", &(shift,).to_variant(), None)
            .await
            .map(|_| ())
    }

    pub fn posicionar(&self, posicao: Position) {
        let nome = match posicao {
            Position::BottomRight => "bottom-right",
            Position::NearPointer => "near-pointer",
        };
        let pedido = self.chamar("SetPosition", &(nome,).to_variant(), None);
        glib::spawn_future_local(async move {
            if let Err(e) = pedido.await {
                eprintln!("supercola: a extensão do GNOME não respondeu: {e}");
            }
        });
    }

    async fn ler(&self, mime: &str) -> Result<Vec<u8>, glib::Error> {
        let tipo = glib::VariantTy::new("(ay)").expect("tipo válido");
        let resposta = self
            .chamar("ReadSelection", &(mime,).to_variant(), Some(tipo))
            .await?;
        Ok(resposta.child_value(0).data().to_vec())
    }

    /// A extensão grava um formato só, então vai o primeiro, que é o principal.
    fn gravar(&self, oferta: &Oferta) {
        let Some((mime, dados)) = oferta.iter().find(|(m, _)| m != model::MIME_TRANSIENT) else {
            return;
        };
        let bytes = glib::Bytes::from(dados.as_slice());
        let dados = glib::Variant::from_bytes::<Vec<u8>>(&bytes);
        let args = glib::Variant::tuple_from_iter([mime.to_variant(), dados]);
        let pedido = self.chamar("SetSelection", &args, None);
        glib::spawn_future_local(async move {
            if let Err(e) = pedido.await {
                eprintln!("supercola: não consegui gravar no clipboard: {e}");
            }
        });
    }

    pub fn clipboard(&self) -> Handle {
        let shell = self.clone();
        Handle::novo("gnome-shell", move |oferta| shell.gravar(&oferta))
    }
}

async fn ler_selecao(shell: Shell, formatos: Vec<(String, String)>, sink: Sink) {
    let mut sabores = Vec::new();
    for (canonico, oferecido) in formatos {
        match shell.ler(&oferecido).await {
            Ok(dados) if dados.len() <= MAX_ITEM_BYTES => {
                if !dados.is_empty() {
                    sabores.push(Flavor::new(canonico, dados));
                }
            }
            Ok(_) | Err(_) => {
                sink(Event::Skipped);
                return;
            }
        }
    }
    if !sabores.is_empty() {
        sink(Event::Selection(sabores));
    }
}

fn nova_selecao(shell: &Shell, formatos: &[String], sink: &Sink) {
    match filter::decidir(formatos) {
        Decisao::Ignorar => {}
        Decisao::Segredo => sink(Event::Secret),
        Decisao::Ler(formatos) => {
            glib::spawn_future_local(ler_selecao(shell.clone(), formatos, sink.clone()));
        }
    }
}

fn vigiar_dono(conn: &gio::DBusConnection, nome: &str, dono: &Dono) {
    let (apareceu, sumiu) = (dono.clone(), dono.clone());
    gio::bus_watch_name_on_connection(
        conn,
        nome,
        BusNameWatcherFlags::NONE,
        move |_, _, atual| *apareceu.borrow_mut() = Some(atual.to_string()),
        move |_, _| *sumiu.borrow_mut() = None,
    );
}

fn registrar(
    conn: &gio::DBusConnection,
    foco: Focus,
    dono: Dono,
    selecao: impl Fn(&[String]) + 'static,
) -> Result<(), glib::Error> {
    let descricao = gio::DBusNodeInfo::for_xml(DESCRICAO)?;
    let Some(interface) = descricao.lookup_interface(INTERFACE) else {
        return Ok(());
    };
    conn.register_object(CAMINHO, &interface)
        .method_call(move |_, remetente, _, _, metodo, args, chamada| {
            let autorizado = remetente.is_some() && remetente == dono.borrow().as_deref();
            if !autorizado {
                chamada.return_dbus_error(
                    "org.freedesktop.DBus.Error.AccessDenied",
                    "só o ambiente gráfico",
                );
                return;
            }
            match metodo {
                "FocusChanged" => {
                    if let Some((app,)) = args.get::<(String,)>() {
                        if !app.is_empty() {
                            foco.definir(app);
                        }
                    }
                }
                "SelectionChanged" => {
                    if let Some((formatos,)) = args.get::<(Vec<String>,)>() {
                        selecao(&formatos);
                    }
                }
                _ => {}
            }
            chamada.return_value(None);
        })
        .build()?;
    Ok(())
}

pub fn iniciar_gnome(foco: Focus, sink: Sink) -> Shell {
    let shell = Shell {
        conexao: Conexao::default(),
    };
    let s = shell.clone();
    gio::bus_own_name(
        BusType::Session,
        NOME,
        BusNameOwnerFlags::NONE,
        move |conn, _| {
            *s.conexao.borrow_mut() = Some(conn.clone());
            let dono = Dono::default();
            vigiar_dono(&conn, GNOME_SHELL, &dono);
            let (s, sink) = (s.clone(), sink.clone());
            let selecao = move |formatos: &[String]| nova_selecao(&s, formatos, &sink);
            if let Err(e) = registrar(&conn, foco.clone(), dono, selecao) {
                eprintln!("supercola: não consegui registrar o serviço D-Bus: {e}");
            }
        },
        |_, _| {},
        |_, _| eprintln!("supercola: sem o serviço D-Bus, a extensão do GNOME não funciona"),
    );
    shell
}

pub fn iniciar_kde(gtk_app: &Application, foco: Focus) {
    let app = gtk_app.clone();
    gio::bus_own_name(
        BusType::Session,
        NOME,
        BusNameOwnerFlags::NONE,
        move |conn, _| {
            let dono = Dono::default();
            vigiar_dono(&conn, KWIN, &dono);
            if let Err(e) = registrar(&conn, foco.clone(), dono, |_| {}) {
                eprintln!("supercola: não consegui registrar o serviço D-Bus: {e}");
            }
            let c = conn.clone();
            app.connect_shutdown(move |_| descarregar_script(&c));
        },
        |conn, _| {
            glib::spawn_future_local(async move {
                if let Err(e) = carregar_script(&conn).await {
                    eprintln!("supercola: não consegui carregar o script do KWin: {e}");
                }
            });
        },
        |_, _| eprintln!("supercola: sem o serviço D-Bus, o app em foco não será detectado"),
    );
}

async fn chamar_kwin(
    conn: &gio::DBusConnection,
    metodo: &str,
    args: Option<&glib::Variant>,
) -> Result<glib::Variant, glib::Error> {
    conn.call_future(
        Some(KWIN),
        KWIN_SCRIPTS,
        KWIN_INTERFACE,
        metodo,
        args,
        None,
        DBusCallFlags::NONE,
        TEMPO_MS,
    )
    .await
}

/// Um daemon que caiu pode ter deixado o script carregado, então ele sai antes.
async fn carregar_script(conn: &gio::DBusConnection) -> Result<(), glib::Error> {
    let script = crate::data_dir().join(SCRIPT_KWIN);
    chamar_kwin(conn, "unloadScript", Some(&(PLUGIN,).to_variant())).await?;
    let args = (script.to_string_lossy().as_ref(), PLUGIN).to_variant();
    chamar_kwin(conn, "loadScript", Some(&args)).await?;
    chamar_kwin(conn, "start", None).await?;
    Ok(())
}

fn descarregar_script(conn: &gio::DBusConnection) {
    let _ = conn.call_sync(
        Some(KWIN),
        KWIN_SCRIPTS,
        KWIN_INTERFACE,
        "unloadScript",
        Some(&(PLUGIN,).to_variant()),
        None,
        DBusCallFlags::NONE,
        TEMPO_MS,
        gio::Cancellable::NONE,
    );
}

/// `state` 1 é uma extensão ativa, e uma extensão desconhecida responde um mapa vazio.
pub fn extensao_ativa() -> Result<bool, String> {
    let conn =
        gio::bus_get_sync(BusType::Session, gio::Cancellable::NONE).map_err(|e| e.to_string())?;
    let resposta = conn
        .call_sync(
            Some(GNOME_SHELL),
            "/org/gnome/Shell",
            "org.gnome.Shell.Extensions",
            "GetExtensionInfo",
            Some(&(EXTENSAO,).to_variant()),
            None,
            DBusCallFlags::NONE,
            TEMPO_MS,
            gio::Cancellable::NONE,
        )
        .map_err(|e| e.to_string())?;
    let dados = glib::VariantDict::new(Some(&resposta.child_value(0)));
    Ok(dados.lookup::<f64>("state").ok().flatten() == Some(1.0))
}

/// O GNOME desliga as extensões depois de uma queda do Shell, e fica assim até alguém ligar.
pub fn extensoes_desligadas() -> bool {
    gio::SettingsSchemaSource::default()
        .and_then(|fonte| fonte.lookup("org.gnome.shell", true))
        .is_some_and(|_| gio::Settings::new("org.gnome.shell").boolean("disable-user-extensions"))
}
