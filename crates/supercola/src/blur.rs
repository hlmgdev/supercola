//! Fundo translúcido: pede ao compositor o desfoque atrás do cartão.
//!
//! O protocolo precisa da `wl_surface` do GTK. Por isso, ao contrário dos protocolos de
//! `supercola-wayland`, este módulo usa a conexão dele, na thread principal.
use std::cell::RefCell;

use gdk4_wayland::prelude::*;
use gdk4_wayland::{WaylandDisplay, WaylandSurface};
use gtk4::gdk;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_region::WlRegion;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::{self as gerente, Capability, ExtBackgroundEffectManagerV1},
    ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1 as Efeito,
};

/// Raio dos cantos do cartão em `data/style.css`.
const RAIO: i32 = 8;

#[derive(Default)]
struct State {
    tem_desfoque: bool,
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

impl Dispatch<ExtBackgroundEffectManagerV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtBackgroundEffectManagerV1,
        event: gerente::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let gerente::Event::Capabilities {
            flags: WEnum::Value(flags),
        } = event
        {
            state.tem_desfoque = flags.contains(Capability::Blur);
        }
    }
}

delegate_noop!(State: ignore WlCompositor);
delegate_noop!(State: ignore WlRegion);
delegate_noop!(State: ignore Efeito);

pub struct Desfoque {
    conn: Connection,
    queue: EventQueue<State>,
    gerente: ExtBackgroundEffectManagerV1,
    compositor: WlCompositor,
    efeito: RefCell<Option<Efeito>>,
}

/// `wl_region` só aceita retângulos: os cantos arredondados viram faixas de 1 px.
fn faixas(largura: i32, altura: i32, raio: i32) -> Vec<(i32, i32, i32, i32)> {
    let mut saida = vec![(0, raio, largura, altura - 2 * raio)];
    for i in 0..raio {
        let d = f64::from(raio - i) - 0.5;
        let recuo = raio - (f64::from(raio * raio) - d * d).sqrt().round() as i32;
        saida.push((recuo, i, largura - 2 * recuo, 1));
        saida.push((recuo, altura - 1 - i, largura - 2 * recuo, 1));
    }
    saida
}

impl Desfoque {
    /// `None` se o compositor não oferece o efeito, e aí o fundo fica sólido.
    pub fn novo(display: &gdk::Display) -> Option<Self> {
        let wl_display = display.downcast_ref::<WaylandDisplay>()?.wl_display()?;
        let conn = Connection::from_backend(wl_display.backend().upgrade()?);
        let (globals, mut queue) = registry_queue_init::<State>(&conn).ok()?;
        let qh = queue.handle();
        let gerente: ExtBackgroundEffectManagerV1 = globals.bind(&qh, 1..=1, ()).ok()?;
        let compositor: WlCompositor = globals.bind(&qh, 1..=4, ()).ok()?;
        let mut state = State::default();
        queue.roundtrip(&mut state).ok()?;
        state.tem_desfoque.then_some(Self {
            conn,
            queue,
            gerente,
            compositor,
            efeito: RefCell::default(),
        })
    }

    /// Desfoca o retângulo do cartão. Vale a partir do próximo quadro do GTK.
    pub fn aplicar(&self, superficie: &gdk::Surface, x: i32, y: i32, largura: i32, altura: i32) {
        let wl_surface = superficie
            .downcast_ref::<WaylandSurface>()
            .and_then(|s| s.wl_surface());
        let Some(wl_surface) = wl_surface else { return };
        let qh = self.queue.handle();
        let mut efeito = self.efeito.borrow_mut();
        let efeito =
            efeito.get_or_insert_with(|| self.gerente.get_background_effect(&wl_surface, &qh, ()));
        let regiao = self.compositor.create_region(&qh, ());
        for (dx, dy, l, a) in faixas(largura, altura, RAIO) {
            regiao.add(x + dx, y + dy, l, a);
        }
        efeito.set_blur_region(Some(&regiao));
        regiao.destroy();
        let _ = self.conn.flush();
    }

    /// Precisa vir antes de esconder a janela, que destrói a `wl_surface`.
    pub fn remover(&self) {
        if let Some(efeito) = self.efeito.borrow_mut().take() {
            efeito.destroy();
            let _ = self.conn.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faixas_cobrem_o_cartao_sem_os_cantos() {
        let f = faixas(360, 400, 8);
        assert_eq!(f[0], (0, 8, 360, 384));
        // 8 faixas em cima e 8 embaixo, além do miolo.
        assert_eq!(f.len(), 17);
        let topo = f.iter().find(|r| r.1 == 0).unwrap();
        assert!(topo.0 > 0 && topo.2 < 360);
        let ultima = f.iter().find(|r| r.1 == 7).unwrap();
        assert_eq!(ultima.0, 0);
        assert!(f.iter().any(|r| r.1 == 399));
    }
}
