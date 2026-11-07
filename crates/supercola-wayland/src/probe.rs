//! Lista os globais do compositor e identifica o ambiente gráfico.
use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};

#[derive(Default)]
struct Globais(Vec<(String, u32)>);

impl Dispatch<wl_registry::WlRegistry, ()> for Globais {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            interface, version, ..
        } = event
        {
            state.0.push((interface, version));
        }
    }
}

pub fn globals() -> crate::R<Vec<(String, u32)>> {
    let conn = Connection::connect_to_env()?;
    let mut queue = conn.new_event_queue();
    conn.display().get_registry(&queue.handle(), ());
    let mut g = Globais::default();
    queue.roundtrip(&mut g)?;
    Ok(g.0)
}

/// Ambiente em uso, para escolher os caminhos de painel, foco e atalho.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ambiente {
    Cosmic,
    Kde,
    Gnome,
    Outro,
}

impl Ambiente {
    pub fn detectar() -> Self {
        Self::de(&globals().unwrap_or_default())
    }

    /// Só globais exclusivos de cada compositor: `XDG_CURRENT_DESKTOP` pode vir herdado, e
    /// o wlroots também tem protocolos `org_kde_kwin_*`.
    fn de(globais: &[(String, u32)]) -> Self {
        let tem = |prefixo: &str| globais.iter().any(|(nome, _)| nome.starts_with(prefixo));
        if tem("zcosmic_") {
            Self::Cosmic
        } else if tem("org_kde_plasma_shell") {
            Self::Kde
        } else if tem("gtk_shell1") && !tem("zwlr_layer_shell") {
            Self::Gnome
        } else {
            Self::Outro
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn globais(nomes: &[&str]) -> Vec<(String, u32)> {
        nomes.iter().map(|n| ((*n).to_string(), 1)).collect()
    }

    #[test]
    fn cosmic_pelos_protocolos_proprios() {
        let g = globais(&["wl_seat", "zcosmic_toplevel_info_v1"]);
        assert_eq!(Ambiente::de(&g), Ambiente::Cosmic);
    }

    #[test]
    fn kde_pelo_shell_do_plasma() {
        let g = globais(&[
            "org_kde_kwin_server_decoration_manager",
            "org_kde_plasma_shell",
        ]);
        assert_eq!(Ambiente::de(&g), Ambiente::Kde);
    }

    #[test]
    fn sway_tem_protocolo_do_kde_e_nao_e_kde() {
        let g = globais(&[
            "org_kde_kwin_server_decoration_manager",
            "zwlr_layer_shell_v1",
        ]);
        assert_eq!(Ambiente::de(&g), Ambiente::Outro);
    }

    #[test]
    fn gnome_pelo_protocolo_do_mutter() {
        let g = globais(&["wl_seat", "gtk_shell1"]);
        assert_eq!(Ambiente::de(&g), Ambiente::Gnome);
    }

    #[test]
    fn gtk_shell_com_layer_shell_nao_e_gnome() {
        let g = globais(&["gtk_shell1", "zwlr_layer_shell_v1"]);
        assert_eq!(Ambiente::de(&g), Ambiente::Outro);
    }
}
