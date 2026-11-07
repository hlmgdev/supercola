//! `supercola --doctor`: diagnóstico sem exibir conteúdo do clipboard.
use supercola_core::config::Config;
use supercola_core::ipc::socket_path;

const NECESSARIOS: &[(&[&str], &str)] = &[
    (
        &[
            "ext_data_control_manager_v1",
            "zwlr_data_control_manager_v1",
        ],
        "monitorar o clipboard",
    ),
    (&["zwlr_layer_shell_v1"], "painel flutuante"),
    (&["zwp_virtual_keyboard_manager_v1"], "colar sozinho"),
    (
        &[
            "zcosmic_toplevel_info_v1",
            "zwlr_foreign_toplevel_manager_v1",
        ],
        "detectar terminais",
    ),
];

pub fn run() {
    println!("supercola {}", env!("CARGO_PKG_VERSION"));
    println!(
        "sessão: {}",
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "?".into())
    );
    println!(
        "GTK: {}.{}.{}",
        gtk4::major_version(),
        gtk4::minor_version(),
        gtk4::micro_version()
    );
    match supercola_wayland::probe::globals() {
        Ok(globais) => {
            for (nomes, papel) in NECESSARIOS {
                let achado = globais.iter().find(|(g, _)| nomes.contains(&g.as_str()));
                match achado {
                    Some((g, v)) => println!("OK      {papel}: {g} v{v}"),
                    None => println!("AUSENTE {papel}: {}", nomes.join(" ou ")),
                }
            }
        }
        Err(e) => println!("ERRO    sem conexão Wayland: {e}"),
    }
    let (config, erro) = Config::load();
    println!(
        "config: {} (gravação via {:?})",
        Config::path().display(),
        config.write_backend
    );
    if let Some(e) = erro {
        println!("ERRO    config inválida, usando padrões: {e}");
    }
    let vivo = std::os::unix::net::UnixStream::connect(socket_path()).is_ok();
    println!("daemon: {}", if vivo { "rodando" } else { "parado" });
}
