//! Grava os atalhos do COSMIC com backup. Nunca sobrescreve um atalho alheio.
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use supercola_core::shortcut::{merge, Merge};
use supercola_core::xdg;
use supercola_wayland::probe::Ambiente;

use crate::app::App;

/// `Super+.` é o zoom do COSMIC; `Super+;` também abre o painel de emoji no Windows.
const ATALHOS: &[(&str, &str, &str)] = &[
    ("Super+V", "v", "supercola-toggle"),
    ("Super+;", "semicolon", "supercola-toggle home"),
];

fn custom_path() -> PathBuf {
    xdg::config_home().join("cosmic/com.system76.CosmicSettings.Shortcuts/v1/custom")
}

pub fn marcador() -> PathBuf {
    xdg::data_dir().join("shortcut-done")
}

pub fn is_cosmic() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|d| d.to_ascii_uppercase().contains("COSMIC"))
}

fn registrar(nome: &str, tecla: &str, comando: &str) -> Result<String, String> {
    let caminho = custom_path();
    let existente = fs::read_to_string(&caminho).ok();
    match merge(existente.as_deref(), "Super", tecla, comando) {
        Merge::AlreadySet => Ok(format!("{nome} já está registrado.")),
        Merge::Conflict(acao) => Err(format!(
            "{nome} já está em uso ({acao}). Nada foi alterado: troque em Configurações > Teclado."
        )),
        Merge::Invalid => Err(format!(
            "{} tem um formato inesperado. Nada foi alterado.",
            caminho.display()
        )),
        Merge::Updated(novo) => {
            if let Some(pai) = caminho.parent() {
                fs::create_dir_all(pai).map_err(|e| e.to_string())?;
            }
            if existente.is_some() {
                fs::copy(&caminho, caminho.with_extension("supercola-backup"))
                    .map_err(|e| e.to_string())?;
            }
            // Troca atômica: o cosmic-settings-daemon observa este arquivo.
            let tmp = caminho.with_extension("supercola-tmp");
            fs::write(&tmp, novo).map_err(|e| e.to_string())?;
            fs::rename(&tmp, &caminho).map_err(|e| e.to_string())?;
            Ok(format!("{nome} registrado."))
        }
    }
}

/// Tenta todos, mesmo que um falhe. `Err` só quando algum não pôde ser registrado.
pub fn install() -> Result<String, String> {
    match Ambiente::detectar() {
        Ambiente::Kde => return crate::atalho_kde::install(),
        Ambiente::Gnome => return crate::atalho_gnome::install(),
        _ => {}
    }
    let resultados: Vec<_> = ATALHOS
        .iter()
        .map(|(nome, tecla, comando)| registrar(nome, tecla, comando))
        .collect();
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

/// Só o que uma troca aceita pelo usuário mudou. No COSMIC, nada é trocado.
pub fn restore() -> Result<String, String> {
    match Ambiente::detectar() {
        Ambiente::Kde => crate::atalho_kde::restaurar(),
        Ambiente::Gnome => crate::atalho_gnome::restaurar(),
        _ => Ok("Nenhum atalho foi trocado.".to_string()),
    }
}

pub fn marcar() {
    if let Some(pai) = marcador().parent() {
        let _ = fs::create_dir_all(pai);
    }
    let _ = fs::write(marcador(), b"");
}

/// Na primeira execução, registra os atalhos uma única vez.
pub fn first_run(app: &Rc<App>) {
    if marcador().exists() {
        return;
    }
    match app.ambiente {
        Ambiente::Kde => return crate::atalho_kde::primeira_execucao(app),
        Ambiente::Gnome => return crate::atalho_gnome::primeira_execucao(app),
        _ => {}
    }
    if !is_cosmic() {
        return;
    }
    match install() {
        Ok(msg) | Err(msg) => eprintln!("supercola: {msg}"),
    }
    marcar();
}
