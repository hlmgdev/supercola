//! `supercola --doctor`: diagnóstico sem exibir conteúdo do clipboard.
use supercola_core::config::Config;
use supercola_core::ipc::socket_path;
use supercola_wayland::probe::Ambiente;

const FOCO: &str = "detectar terminais";

const NECESSARIOS: &[(&[&str], &str)] = &[
    (
        &[
            "ext_data_control_manager_v1",
            "zwlr_data_control_manager_v1",
        ],
        "monitorar o clipboard",
    ),
    (&["zwlr_layer_shell_v1"], "painel flutuante"),
    (
        &["zwp_virtual_keyboard_manager_v1", "org_kde_kwin_fake_input"],
        "colar sozinho",
    ),
    (
        &[
            "zcosmic_toplevel_info_v1",
            "zwlr_foreign_toplevel_manager_v1",
        ],
        FOCO,
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
    let gnome = Ambiente::detectar() == Ambiente::Gnome;
    if gnome {
        match crate::bridge::extensao_ativa() {
            Ok(true) => println!("OK      extensão do GNOME: ativa"),
            Ok(false) if crate::bridge::extensoes_desligadas() => println!(
                "AUSENTE extensão do GNOME: as extensões estão desligadas, ligue-as em Extensões"
            ),
            Ok(false) => println!("AUSENTE extensão do GNOME: saia e entre de novo na sessão"),
            Err(e) => println!("ERRO    extensão do GNOME: {e}"),
        }
    }
    match supercola_wayland::probe::globals() {
        Ok(_) if gnome => {}
        Ok(globais) => {
            let kde = Ambiente::detectar() == Ambiente::Kde;
            for (nomes, papel) in NECESSARIOS {
                if kde && *papel == FOCO {
                    let script = crate::data_dir().join(crate::bridge::SCRIPT_KWIN);
                    if script.exists() {
                        println!("OK      {papel}: script do KWin");
                    } else {
                        println!("AUSENTE {papel}: {}", script.display());
                    }
                    continue;
                }
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
