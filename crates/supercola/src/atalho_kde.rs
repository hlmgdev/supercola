//! Atalhos no KDE, pelo serviço `kglobalaccel`.
//!
//! Cada atalho é um `.desktop` do pacote, que cria a ação no KDE. O padrão declarado nele
//! só vale com a tecla livre, então a primeira execução confere quem ocupa cada tecla.
//! `Super+V` costuma ser do Klipper, e só muda de dono se o usuário aceitar.
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::gio::{self, BusType, DBusCallFlags};
use gtk4::{glib, prelude::*, AlertDialog};
use supercola_core::shortcut::Substituido;
use supercola_core::xdg;

use crate::app::App;

const SERVICO: &str = "org.kde.kglobalaccel";
const CAMINHO: &str = "/kglobalaccel";
const INTERFACE: &str = "org.kde.KGlobalAccel";
const TEMPO_MS: i32 = 5000;

/// Códigos do Qt: `Qt::META | Qt::Key_V` e `Qt::META | Qt::Key_Semicolon`.
const SUPER_V: i32 = 0x1000_0056;
const SUPER_PONTO_E_VIRGULA: i32 = 0x1000_003b;

const HISTORICO: Acao = Acao("dev.supercola.Toggle.desktop", SUPER_V, "Super+V");
const INICIO: Acao = Acao(
    "dev.supercola.Home.desktop",
    SUPER_PONTO_E_VIRGULA,
    "Super+;",
);
const ACAO_DESKTOP: &str = "_launch";
const NOME: &str = "Super Cola";

/// `.desktop` do pacote, tecla e nome para as mensagens.
struct Acao(&'static str, i32, &'static str);

/// Quem está com a tecla, como o `kglobalaccel` informa.
type Ocupante = (
    String,
    String,
    String,
    String,
    String,
    String,
    Vec<i32>,
    Vec<i32>,
);

fn backup() -> PathBuf {
    xdg::data_dir().join("atalho-substituido")
}

fn conexao() -> Result<gio::DBusConnection, String> {
    gio::bus_get_sync(BusType::Session, gio::Cancellable::NONE).map_err(|e| e.to_string())
}

fn chamar(
    conn: &gio::DBusConnection,
    metodo: &str,
    args: &glib::Variant,
) -> Result<glib::Variant, String> {
    conn.call_sync(
        Some(SERVICO),
        CAMINHO,
        INTERFACE,
        metodo,
        Some(args),
        None,
        DBusCallFlags::NONE,
        TEMPO_MS,
        gio::Cancellable::NONE,
    )
    .map_err(|e| e.to_string())
}

fn donos(conn: &gio::DBusConnection, tecla: i32) -> Result<Vec<Ocupante>, String> {
    let resposta = chamar(conn, "getGlobalShortcutsByKey", &(tecla,).to_variant())?;
    resposta
        .get::<(Vec<Ocupante>,)>()
        .map(|(lista,)| lista)
        .ok_or_else(|| "resposta inesperada do kglobalaccel".to_string())
}

fn definir(
    conn: &gio::DBusConnection,
    componente: &str,
    acao: &str,
    nomes: (&str, &str),
    teclas: &[i32],
) -> Result<(), String> {
    let id = vec![componente, acao, nomes.0, nomes.1];
    chamar(
        conn,
        "setForeignShortcut",
        &(id, teclas.to_vec()).to_variant(),
    )
    .map(|_| ())
}

fn atribuir(conn: &gio::DBusConnection, acao: &Acao) -> Result<(), String> {
    definir(conn, acao.0, ACAO_DESKTOP, (NOME, NOME), &[acao.1])
}

/// Dono da tecla, sem contar o próprio Super Cola.
fn outro_dono(conn: &gio::DBusConnection, acao: &Acao) -> Result<Option<Ocupante>, String> {
    Ok(donos(conn, acao.1)?
        .into_iter()
        .find(|o| !o.2.starts_with("dev.supercola.")))
}

fn registrar(conn: &gio::DBusConnection, acao: &Acao) -> Result<String, String> {
    match outro_dono(conn, acao)? {
        None => {
            atribuir(conn, acao)?;
            Ok(format!("{} registrado.", acao.2))
        }
        Some(o) => Err(format!(
            "{} já está em uso ({}). Nada foi alterado: troque em Configurações do sistema > Atalhos.",
            acao.2, o.1
        )),
    }
}

/// Tenta os dois, mesmo que um falhe, e nunca tira a tecla de outro app.
pub fn install() -> Result<String, String> {
    let conn = conexao()?;
    let resultados = [registrar(&conn, &HISTORICO), registrar(&conn, &INICIO)];
    let falhou = resultados.iter().any(Result::is_err);
    let texto = resultados
        .into_iter()
        .map(|r| r.unwrap_or_else(|e| e))
        .collect::<Vec<_>>()
        .join("\n");
    if falhou {
        Err(texto)
    } else {
        Ok(texto)
    }
}

/// Guarda o que havia antes de mexer: sem backup, a troca não acontece.
fn trocar(conn: &gio::DBusConnection, dono: &Ocupante) -> Result<(), String> {
    let anterior = Substituido {
        componente: dono.2.clone(),
        acao: dono.0.clone(),
        nome_componente: dono.3.clone(),
        nome_acao: dono.1.clone(),
        teclas: dono.6.clone(),
    };
    fs::write(backup(), anterior.para_linha()).map_err(|e| e.to_string())?;
    let restantes: Vec<i32> = dono.6.iter().copied().filter(|t| *t != SUPER_V).collect();
    definir(
        conn,
        &anterior.componente,
        &anterior.acao,
        (&anterior.nome_componente, &anterior.nome_acao),
        &restantes,
    )?;
    atribuir(conn, &HISTORICO)
}

fn perguntar(app: &Rc<App>, conn: gio::DBusConnection, dono: Ocupante) {
    let detalhe = app.tr("atalho_detalhe").replace("{acao}", &dono.1);
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
                if let Err(e) = trocar(&conn, &dono) {
                    eprintln!("supercola: não consegui trocar Super+V: {e}");
                }
            }
            crate::shortcut::marcar();
        },
    );
}

pub fn primeira_execucao(app: &Rc<App>) {
    let conn = match conexao() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("supercola: sem acesso aos atalhos do KDE: {e}");
            return;
        }
    };
    if let Err(e) = registrar(&conn, &INICIO) {
        eprintln!("supercola: {e}");
    }
    match outro_dono(&conn, &HISTORICO) {
        Ok(Some(dono)) => perguntar(app, conn, dono),
        Ok(None) => {
            if let Err(e) = atribuir(&conn, &HISTORICO) {
                eprintln!("supercola: {e}");
            }
            crate::shortcut::marcar();
        }
        Err(e) => eprintln!("supercola: {e}"),
    }
}

/// Devolve a tecla ao app de antes. Sem backup, nada foi trocado.
pub fn restaurar() -> Result<String, String> {
    let Ok(linha) = fs::read_to_string(backup()) else {
        return Ok("Nenhum atalho foi trocado.".to_string());
    };
    let anterior = Substituido::de_linha(&linha)
        .ok_or_else(|| format!("{} tem um formato inesperado.", backup().display()))?;
    let conn = conexao()?;
    definir(&conn, HISTORICO.0, ACAO_DESKTOP, (NOME, NOME), &[])?;
    definir(
        &conn,
        &anterior.componente,
        &anterior.acao,
        (&anterior.nome_componente, &anterior.nome_acao),
        &anterior.teclas,
    )?;
    fs::remove_file(backup()).map_err(|e| e.to_string())?;
    Ok(format!(
        "Super+V voltou para: {}. O Super Cola continua em Super+;.",
        anterior.nome_acao
    ))
}
