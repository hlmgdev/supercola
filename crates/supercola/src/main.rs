//! Daemon e interface do Super Cola: histórico da área de transferência no estilo Win+V.
mod app;
mod blur;
mod doctor;
mod gif;
mod grid;
mod history;
mod home;
mod ipc;
mod panel;
mod settings;
mod shortcut;
mod theme;

use std::path::PathBuf;

use gtk4::glib::ExitCode;
use gtk4::prelude::*;
use gtk4::{gio, Application};

const AJUDA: &str = "\
uso: supercola [opção]

  (nenhuma)            inicia o daemon
  --doctor             verifica protocolos e configuração
  --install-shortcut   registra Super+V no COSMIC
  --version            mostra a versão
";

/// Onde ficam ícones e fontes: `/usr/share/supercola` ou, em testes, `SUPERCOLA_DATA_DIR`.
pub fn data_dir() -> PathBuf {
    std::env::var_os("SUPERCOLA_DATA_DIR")
        .map_or_else(|| PathBuf::from("/usr/share/supercola"), PathBuf::from)
}

fn daemon() -> ExitCode {
    // O painel só existe em Wayland, e terminais como o do VS Code exportam `x11`.
    // Roda antes de qualquer thread.
    std::env::set_var("GDK_BACKEND", "wayland");
    let gtk_app = Application::builder()
        .application_id("dev.supercola.SuperCola")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    gtk_app.connect_startup(|gtk_app| {
        if !gtk4_layer_shell::is_supported() {
            eprintln!(
                "supercola: o compositor não tem layer-shell. Rode dentro de uma sessão COSMIC."
            );
            gtk_app.quit();
            return;
        }
        // Segura o processo vivo sem nenhuma janela aberta.
        std::mem::forget(gtk_app.hold());
        app::App::start(gtk_app);
    });
    gtk_app.connect_activate(|_| {});
    gtk_app.run_with_args::<String>(&[])
}

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        None => daemon(),
        Some("--doctor") => {
            doctor::run();
            ExitCode::SUCCESS
        }
        Some("--install-shortcut") => match shortcut::install() {
            Ok(msg) => {
                println!("{msg}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        Some("--version") => {
            println!("supercola {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some(_) => {
            print!("{AJUDA}");
            ExitCode::SUCCESS
        }
    }
}
