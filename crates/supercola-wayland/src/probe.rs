//! Lista os globais do compositor, para o `supercola --doctor`.
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
