// Incluído em `dc::ext` e `dc::wlr`: os dois protocolos têm a mesma forma.
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::AsFd;
use std::sync::Arc;

use wayland_client::backend::ObjectId;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_registry::WlRegistry, wl_seat::WlSeat};
use wayland_client::{
    delegate_noop, event_created_child, Connection, Dispatch, Proxy, QueueHandle,
};

#[derive(Default)]
struct State {
    offers: HashMap<ObjectId, Vec<String>>,
    data: Option<Arc<String>>,
    done: bool,
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
delegate_noop!(State: ignore WlSeat);
delegate_noop!(State: Manager);

impl Dispatch<Offer, ()> for State {
    fn event(
        state: &mut Self,
        offer: &Offer,
        event: ofr::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ofr::Event::Offer { mime_type } = event {
            state.offers.entry(offer.id()).or_default().push(mime_type);
        }
    }
}

impl Dispatch<Device, ()> for State {
    fn event(
        state: &mut Self,
        _: &Device,
        event: dev::Event,
        _: &(),
        conn: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let dev::Event::Selection { id: Some(offer) } = event else { return };
        let mimes = state.offers.remove(&offer.id()).unwrap_or_default();
        println!("SELECAO mimes={mimes:?}");
        let texto = mimes.iter().find(|m| m.starts_with("text/plain"));
        if let Some(mime) = texto {
            let Ok((mut leitor, escritor)) = std::io::pipe() else { return };
            offer.receive(mime.clone(), escritor.as_fd());
            drop(escritor);
            let _ = conn.flush();
            // Em thread: o dono da seleção pode ser lento. O conteúdo nunca é impresso.
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                if leitor.read_to_end(&mut bytes).is_ok() {
                    println!("DADOS len={} hash={:016x}", bytes.len(), crate::payload::hash(&bytes));
                }
            });
        }
    }

    event_created_child!(State, Device, [dev::EVT_DATA_OFFER_OPCODE => (Offer, ())]);
}

impl Dispatch<Source, ()> for State {
    fn event(
        state: &mut Self,
        _: &Source,
        event: src::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            src::Event::Send { fd, .. } => {
                if let Some(data) = state.data.clone() {
                    std::thread::spawn(move || {
                        let _ = File::from(fd).write_all(data.as_bytes());
                    });
                }
            }
            src::Event::Cancelled => state.done = true,
            _ => {}
        }
    }
}

pub fn watch() -> crate::R {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&conn)?;
    let qh = queue.handle();
    let manager: Manager = globals.bind(&qh, 1..=1, ())?;
    let seat: WlSeat = globals.bind(&qh, 1..=1, ())?;
    let _device = manager.get_data_device(&seat, &qh, ());
    println!("BACKEND {BACKEND}");
    let mut state = State::default();
    loop {
        queue.blocking_dispatch(&mut state)?;
    }
}

/// Oferece `text` como seleção até outro app assumir o clipboard.
pub fn serve(text: Arc<String>) -> crate::R {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&conn)?;
    let qh = queue.handle();
    let manager: Manager = globals.bind(&qh, 1..=1, ())?;
    let seat: WlSeat = globals.bind(&qh, 1..=1, ())?;
    let device = manager.get_data_device(&seat, &qh, ());
    let source = manager.create_data_source(&qh, ());
    for mime in ["text/plain;charset=utf-8", "text/plain", "UTF8_STRING"] {
        source.offer(mime.to_string());
    }
    device.set_selection(Some(&source));
    println!("BACKEND {BACKEND}");
    let mut state = State { data: Some(text), ..State::default() };
    while !state.done {
        queue.blocking_dispatch(&mut state)?;
    }
    Ok(())
}
