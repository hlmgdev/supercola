//! Primeiro uso no GNOME: ativa a extensão e registra os atalhos personalizados.
//!
//! `Super+V` abre as notificações por padrão, então só muda de dono se o usuário aceitar.
//! A tecla `Super+M`, que faz o mesmo, continua lá.
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gtk4::gio::{self, prelude::*, Settings, SettingsSchemaSource};
use gtk4::{glib, AlertDialog};
use supercola_core::xdg;

use crate::app::App;
use crate::bridge::EXTENSAO;

const SHELL: &str = "org.gnome.shell";
const EXTENSOES: &str = "enabled-extensions";
const TECLAS_SHELL: &str = "org.gnome.shell.keybindings";
const NOTIFICACOES: &str = "toggle-message-tray";
const SUPER_V: &str = "<Super>v";
const SUPER_PONTO_E_VIRGULA: &str = "<Super>semicolon";

const MIDIA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const PERSONALIZADOS: &str = "custom-keybindings";
const PERSONALIZADO: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
const RAIZ: &str = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings";

const HISTORICO: Atalho = Atalho("supercola-historico", SUPER_V, "supercola-toggle");
const INICIO: Atalho = Atalho(
    "supercola-inicio",
    SUPER_PONTO_E_VIRGULA,
    "supercola-toggle home",
);

/// Tempo para o GNOME reagir à extensão recém-habilitada.
const ESPERA_EXTENSAO: Duration = Duration::from_secs(2);

/// Pasta nas configurações, tecla e comando.
struct Atalho(&'static str, &'static str, &'static str);

fn backup() -> PathBuf {
    xdg::data_dir().join("atalho-substituido-gnome")
}

/// `Settings::new` encerra o processo se o esquema não existir.
fn configuracao(esquema: &str) -> Result<Settings, String> {
    SettingsSchemaSource::default()
        .and_then(|fonte| fonte.lookup(esquema, true))
        .map(|_| Settings::new(esquema))
        .ok_or_else(|| format!("este GNOME não tem as configurações {esquema}"))
}

fn caminho(atalho: &Atalho) -> String {
    format!("{RAIZ}/{}/", atalho.0)
}

fn registrar(atalho: &Atalho) -> Result<(), String> {
    let midia = configuracao(MIDIA)?;
    let mut lista: Vec<String> = midia
        .strv(PERSONALIZADOS)
        .iter()
        .map(ToString::to_string)
        .collect();
    let meu = caminho(atalho);
    if !lista.contains(&meu) {
        lista.push(meu.clone());
        midia
            .set_strv(PERSONALIZADOS, lista)
            .map_err(|e| e.to_string())?;
    }
    let item = Settings::with_path(PERSONALIZADO, &meu);
    let gravar =
        |chave: &str, valor: &str| item.set_string(chave, valor).map_err(|e| e.to_string());
    gravar("name", "Super Cola")?;
    gravar("command", atalho.2)?;
    gravar("binding", atalho.1)?;
    gio::Settings::sync();
    Ok(())
}

fn remover(atalho: &Atalho) -> Result<(), String> {
    let midia = configuracao(MIDIA)?;
    let meu = caminho(atalho);
    let lista: Vec<String> = midia
        .strv(PERSONALIZADOS)
        .iter()
        .map(ToString::to_string)
        .filter(|p| *p != meu)
        .collect();
    midia
        .set_strv(PERSONALIZADOS, lista)
        .map_err(|e| e.to_string())?;
    let item = Settings::with_path(PERSONALIZADO, &meu);
    for chave in ["name", "command", "binding"] {
        item.reset(chave);
    }
    gio::Settings::sync();
    Ok(())
}

fn notificacoes_na_tecla() -> Result<bool, String> {
    Ok(configuracao(TECLAS_SHELL)?
        .strv(NOTIFICACOES)
        .iter()
        .any(|t| t.eq_ignore_ascii_case(SUPER_V)))
}

/// Tenta os dois, e nunca tira a tecla das notificações.
pub fn install() -> Result<String, String> {
    registrar(&INICIO)?;
    if notificacoes_na_tecla()? {
        return Err(
            "Super+; registrado. Super+V abre as notificações: nada foi alterado.".to_string(),
        );
    }
    registrar(&HISTORICO)?;
    Ok("Super+V e Super+; registrados.".to_string())
}

/// Guarda o que havia antes de mexer: sem backup, a troca não acontece.
fn trocar() -> Result<(), String> {
    let teclas = configuracao(TECLAS_SHELL)?;
    let atuais: Vec<String> = teclas
        .strv(NOTIFICACOES)
        .iter()
        .map(ToString::to_string)
        .collect();
    fs::write(backup(), atuais.join("\n")).map_err(|e| e.to_string())?;
    let restantes: Vec<&String> = atuais
        .iter()
        .filter(|t| !t.eq_ignore_ascii_case(SUPER_V))
        .collect();
    teclas
        .set_strv(NOTIFICACOES, restantes)
        .map_err(|e| e.to_string())?;
    gio::Settings::sync();
    registrar(&HISTORICO)
}

fn perguntar(app: &Rc<App>) {
    let detalhe = app
        .tr("atalho_detalhe")
        .replace("{acao}", app.tr("atalho_notificacoes"));
    let dialogo = AlertDialog::builder()
        .message(app.tr("atalho_titulo"))
        .detail(detalhe)
        .buttons([app.tr("atalho_manter"), app.tr("atalho_trocar")])
        .cancel_button(0)
        .default_button(1)
        .modal(true)
        .build();
    let segurar = app.gtk_app.hold();
    dialogo.choose(
        None::<&gtk4::Window>,
        gio::Cancellable::NONE,
        move |resposta| {
            let _segurar = segurar;
            if matches!(resposta, Ok(1)) {
                if let Err(e) = trocar() {
                    eprintln!("supercola: não consegui trocar Super+V: {e}");
                }
            }
            crate::shortcut::marcar();
        },
    );
}

/// Devolve o que a extensão precisa para carregar: `true` se ela já estava habilitada.
fn habilitar_extensao() -> Result<bool, String> {
    let shell = configuracao(SHELL)?;
    let mut lista: Vec<String> = shell
        .strv(EXTENSOES)
        .iter()
        .map(ToString::to_string)
        .collect();
    if lista.iter().any(|e| e == EXTENSAO) {
        return Ok(true);
    }
    lista.push(EXTENSAO.to_string());
    shell
        .set_strv(EXTENSOES, lista)
        .map_err(|e| e.to_string())?;
    gio::Settings::sync();
    Ok(false)
}

fn avisar_nova_sessao(app: &Rc<App>) {
    let dialogo = AlertDialog::builder()
        .message(app.tr("extensao_titulo"))
        .detail(app.tr("extensao_detalhe"))
        .buttons([app.tr("extensao_ok")])
        .modal(true)
        .build();
    let segurar = app.gtk_app.hold();
    dialogo.choose(None::<&gtk4::Window>, gio::Cancellable::NONE, move |_| {
        let _segurar = segurar;
    });
}

pub fn primeira_execucao(app: &Rc<App>) {
    match habilitar_extensao() {
        Ok(true) => registrar_atalhos(app),
        // A extensão já instalada ao entrar na sessão liga na hora. Uma mais nova que a
        // sessão só carrega na próxima, e aí o usuário precisa saber.
        Ok(false) => {
            let app = app.clone();
            glib::timeout_add_local_once(ESPERA_EXTENSAO, move || {
                if crate::bridge::extensao_ativa().unwrap_or(false) {
                    registrar_atalhos(&app);
                } else {
                    avisar_nova_sessao(&app);
                }
            });
        }
        Err(e) => eprintln!("supercola: não consegui ativar a extensão: {e}"),
    }
}

fn registrar_atalhos(app: &Rc<App>) {
    if let Err(e) = registrar(&INICIO) {
        eprintln!("supercola: {e}");
    }
    match notificacoes_na_tecla() {
        Ok(true) => perguntar(app),
        Ok(false) => {
            if let Err(e) = registrar(&HISTORICO) {
                eprintln!("supercola: {e}");
            }
            crate::shortcut::marcar();
        }
        Err(e) => eprintln!("supercola: {e}"),
    }
}

/// Devolve `Super+V` às notificações. Sem backup, nada foi trocado.
pub fn restaurar() -> Result<String, String> {
    let Ok(texto) = fs::read_to_string(backup()) else {
        return Ok("Nenhum atalho foi trocado.".to_string());
    };
    let originais: Vec<&str> = texto.lines().filter(|l| !l.is_empty()).collect();
    if originais.is_empty() {
        return Err(format!("{} tem um formato inesperado.", backup().display()));
    }
    remover(&HISTORICO)?;
    configuracao(TECLAS_SHELL)?
        .set_strv(NOTIFICACOES, originais)
        .map_err(|e| e.to_string())?;
    gio::Settings::sync();
    fs::remove_file(backup()).map_err(|e| e.to_string())?;
    Ok("Super+V voltou para as notificações. O Super Cola continua em Super+;.".to_string())
}
