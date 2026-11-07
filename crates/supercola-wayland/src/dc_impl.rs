// Incluído nos módulos `ext` e `wlr`: os dois protocolos têm a mesma forma.
use std::collections::HashMap;
use std::fs::File;
use std::io::Write as _;
use std::os::fd::AsFd;

use wayland_client::backend::{ObjectId, WaylandError};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_registry::WlRegistry, wl_seat::WlSeat};
use wayland_client::{
    delegate_noop, event_created_child, Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};

struct State {
    offers: HashMap<ObjectId, Vec<String>>,
    sources: HashMap<ObjectId, Oferta>,
    sink: Sink,
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
        if mimes.iter().any(|m| m == supercola_core::model::MIME_TRANSIENT) {
            offer.destroy();
            return;
        }
        if supercola_core::filter::is_secret(&mimes) {
            offer.destroy();
            (state.sink)(Event::Secret);
            return;
        }
        let mut canos = Vec::new();
        for (canonico, oferecido) in supercola_core::model::wanted(&mimes) {
            let Ok((leitor, escritor)) = std::io::pipe() else { continue };
            offer.receive(oferecido, escritor.as_fd());
            canos.push((canonico, leitor));
        }
        offer.destroy();
        let _ = conn.flush();
        // O dono da seleção pode ser lento, e a leitura não pode travar o protocolo.
        let sink = state.sink.clone();
        std::thread::spawn(move || ler_tudo(canos, sink));
    }

    event_created_child!(State, Device, [dev::EVT_DATA_OFFER_OPCODE => (Offer, ())]);
}

impl Dispatch<Source, ()> for State {
    fn event(
        state: &mut Self,
        source: &Source,
        event: src::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            src::Event::Send { mime_type, fd } => {
                let dados = state
                    .sources
                    .get(&source.id())
                    .and_then(|o| o.iter().find(|(m, _)| *m == mime_type))
                    .map(|(_, d)| d.clone());
                if let Some(dados) = dados {
                    std::thread::spawn(move || {
                        let _ = File::from(fd).write_all(&dados);
                    });
                }
            }
            src::Event::Cancelled => {
                state.sources.remove(&source.id());
                source.destroy();
            }
            _ => {}
        }
    }
}

fn gravar(
    state: &mut State,
    oferta: Oferta,
    manager: &Manager,
    device: &Device,
    qh: &QueueHandle<State>,
) {
    let source = manager.create_data_source(qh, ());
    for (mime, _) in &oferta {
        source.offer(mime.clone());
    }
    device.set_selection(Some(&source));
    state.sources.insert(source.id(), oferta);
}

fn laco(
    conn: Connection,
    mut queue: EventQueue<State>,
    mut state: State,
    manager: Manager,
    device: Device,
    rx: mpsc::Receiver<Oferta>,
    mut wake: std::io::PipeReader,
) -> crate::R {
    let qh = queue.handle();
    let fd_wake = wake.as_raw_fd();
    loop {
        queue.dispatch_pending(&mut state)?;
        conn.flush()?;
        let Some(leitura) = queue.prepare_read() else { continue };
        let fd_wl = leitura.connection_fd().as_raw_fd();
        let mut pfds = [
            libc::pollfd { fd: fd_wl, events: libc::POLLIN, revents: 0 },
            libc::pollfd { fd: fd_wake, events: libc::POLLIN, revents: 0 },
        ];
        // SAFETY: `pfds` vive durante a chamada; os dois descritores estão abertos.
        let r = unsafe { libc::poll(pfds.as_mut_ptr(), 2, -1) };
        if r < 0 {
            let e = std::io::Error::last_os_error();
            if e.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(e.into());
        }
        if pfds[0].revents != 0 {
            match leitura.read() {
                Ok(_) => {}
                // Acordou sem nada para ler: não é erro, e a thread não pode morrer por isso.
                Err(WaylandError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.into()),
            }
        } else {
            drop(leitura);
        }
        if pfds[1].revents != 0 {
            let mut lixo = [0u8; 64];
            let _ = wake.read(&mut lixo);
            while let Ok(oferta) = rx.try_recv() {
                gravar(&mut state, oferta, &manager, &device, &qh);
            }
        }
    }
}

pub fn start(sink: Sink) -> crate::R<Handle> {
    let conn = Connection::connect_to_env()?;
    let (globals, queue) = registry_queue_init::<State>(&conn)?;
    let qh = queue.handle();
    let manager: Manager = globals.bind(&qh, VERSAO..=VERSAO, ())?;
    let seat: WlSeat = globals.bind(&qh, 1..=1, ())?;
    let device = manager.get_data_device(&seat, &qh, ());
    let (tx, rx) = mpsc::channel();
    let (leitor, escritor) = std::io::pipe()?;
    let state = State { offers: HashMap::new(), sources: HashMap::new(), sink: sink.clone() };
    std::thread::Builder::new().name(BACKEND.into()).spawn(move || {
        if let Err(e) = laco(conn, queue, state, manager, device, rx, leitor) {
            eprintln!("supercola: conexão do clipboard caiu: {e}");
        }
    })?;
    Ok(Handle { tx, wake: escritor, backend: BACKEND })
}
