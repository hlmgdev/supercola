//! Clique por ponteiro virtual, para testes em compositor sem dispositivo real.
use std::time::Duration;

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_pointer::ButtonState;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{delegate_noop, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1 as Manager,
    zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1 as Pointer,
};

const BTN_LEFT: u32 = 0x110;
const LARGURA: u32 = 1920;
const ALTURA: u32 = 1080;

struct State;

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
delegate_noop!(State: Pointer);

/// Deixa um ponteiro parado em `(x, y)` por `ms`, para superfícies que abrem depois.
pub fn hover(x: u32, y: u32, ms: u64) -> crate::R {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&conn)?;
    let qh = queue.handle();
    let manager: Manager = globals.bind(&qh, 1..=1, ())?;
    let seat: WlSeat = globals.bind(&qh, 1..=1, ())?;
    let ponteiro = manager.create_virtual_pointer(Some(&seat), &qh, ());
    queue.roundtrip(&mut State)?;
    std::thread::sleep(Duration::from_millis(150));
    ponteiro.motion_absolute(0, x, y, LARGURA, ALTURA);
    ponteiro.frame();
    queue.roundtrip(&mut State)?;
    std::thread::sleep(Duration::from_millis(ms));
    Ok(())
}

pub fn click(x: u32, y: u32) -> crate::R {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&conn)?;
    let qh = queue.handle();
    let manager: Manager = globals.bind(&qh, 1..=1, ())?;
    let seat: WlSeat = globals.bind(&qh, 1..=1, ())?;
    let ponteiro = manager.create_virtual_pointer(Some(&seat), &qh, ());
    let pausa = Duration::from_millis(150);

    // O app precisa perceber o novo ponteiro antes do primeiro evento.
    queue.roundtrip(&mut State)?;
    std::thread::sleep(pausa);
    ponteiro.motion_absolute(0, x, y, LARGURA, ALTURA);
    ponteiro.frame();
    queue.roundtrip(&mut State)?;
    std::thread::sleep(pausa);
    ponteiro.button(10, BTN_LEFT, ButtonState::Pressed);
    ponteiro.frame();
    queue.roundtrip(&mut State)?;
    std::thread::sleep(Duration::from_millis(40));
    ponteiro.button(50, BTN_LEFT, ButtonState::Released);
    ponteiro.frame();
    queue.roundtrip(&mut State)?;
    std::thread::sleep(pausa);
    ponteiro.destroy();
    conn.flush()?;
    Ok(())
}
