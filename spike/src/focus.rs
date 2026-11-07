//! Descobre o app em foco: protocolo do COSMIC, com o `wlr` como reserva (sway).
use std::collections::HashMap;

use cosmic_protocols::toplevel_info::v1::client::{
    zcosmic_toplevel_handle_v1::{self as chandle, ZcosmicToplevelHandleV1 as CosmicHandle},
    zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1 as Info,
};
use wayland_client::backend::ObjectId;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{event_created_child, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self as fhandle, ExtForeignToplevelHandleV1 as FHandle},
    ext_foreign_toplevel_list_v1::{self as flist, ExtForeignToplevelListV1 as FList},
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self as whandle, ZwlrForeignToplevelHandleV1 as WHandle},
    zwlr_foreign_toplevel_manager_v1::{
        self as wmanager, ZwlrForeignToplevelManagerV1 as WManager,
    },
};

const ACTIVATED: u32 = 2; // mesmo valor nos dois protocolos

#[derive(Default)]
struct Janela {
    app_id: String,
    ativa: bool,
}

#[derive(Default)]
struct State {
    janelas: HashMap<ObjectId, Janela>,
    cosmic_para_foreign: HashMap<ObjectId, ObjectId>,
    info: Option<Info>,
    em_foco: Option<String>,
}

impl State {
    fn relatar(&mut self) {
        let atual = self
            .janelas
            .values()
            .find(|j| j.ativa)
            .map(|j| j.app_id.clone());
        if atual != self.em_foco {
            println!("FOCO app_id={}", atual.as_deref().unwrap_or("(nenhum)"));
            self.em_foco = atual;
        }
    }
}

fn tem_ativada(bytes: &[u8]) -> bool {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .any(|c| u32::from_ne_bytes(*c) == ACTIVATED)
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

impl Dispatch<Info, ()> for State {
    fn event(
        _: &mut Self,
        _: &Info,
        _: <Info as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<FList, ()> for State {
    fn event(
        state: &mut Self,
        _: &FList,
        event: flist::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let flist::Event::Toplevel { toplevel } = event {
            state.janelas.insert(toplevel.id(), Janela::default());
            if let Some(info) = &state.info {
                let cosmic = info.get_cosmic_toplevel(&toplevel, qh, ());
                state.cosmic_para_foreign.insert(cosmic.id(), toplevel.id());
            }
        }
    }

    event_created_child!(State, FList, [flist::EVT_TOPLEVEL_OPCODE => (FHandle, ())]);
}

impl Dispatch<FHandle, ()> for State {
    fn event(
        state: &mut Self,
        handle: &FHandle,
        event: fhandle::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            fhandle::Event::AppId { app_id } => {
                if let Some(j) = state.janelas.get_mut(&handle.id()) {
                    j.app_id = app_id;
                }
            }
            fhandle::Event::Closed => {
                state.janelas.remove(&handle.id());
                state.relatar();
            }
            fhandle::Event::Done => state.relatar(),
            _ => {}
        }
    }
}

impl Dispatch<CosmicHandle, ()> for State {
    fn event(
        state: &mut Self,
        handle: &CosmicHandle,
        event: chandle::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let chandle::Event::State { state: bytes } = event {
            let Some(foreign) = state.cosmic_para_foreign.get(&handle.id()) else {
                return;
            };
            if let Some(j) = state.janelas.get_mut(foreign) {
                j.ativa = tem_ativada(&bytes);
            }
        }
    }
}

impl Dispatch<WManager, ()> for State {
    fn event(
        state: &mut Self,
        _: &WManager,
        event: wmanager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wmanager::Event::Toplevel { toplevel } = event {
            state.janelas.insert(toplevel.id(), Janela::default());
        }
    }

    event_created_child!(State, WManager, [wmanager::EVT_TOPLEVEL_OPCODE => (WHandle, ())]);
}

impl Dispatch<WHandle, ()> for State {
    fn event(
        state: &mut Self,
        handle: &WHandle,
        event: whandle::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            whandle::Event::AppId { app_id } => {
                if let Some(j) = state.janelas.get_mut(&handle.id()) {
                    j.app_id = app_id;
                }
            }
            whandle::Event::State { state: bytes } => {
                if let Some(j) = state.janelas.get_mut(&handle.id()) {
                    j.ativa = tem_ativada(&bytes);
                }
            }
            whandle::Event::Closed => {
                state.janelas.remove(&handle.id());
                state.relatar();
            }
            whandle::Event::Done => state.relatar(),
            _ => {}
        }
    }
}

pub fn run() -> crate::R {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&conn)?;
    let qh = queue.handle();
    let mut state = State::default();

    let cosmic: Option<(FList, Info)> = match (
        globals.bind::<FList, _, _>(&qh, 1..=1, ()),
        globals.bind::<Info, _, _>(&qh, 2..=3, ()),
    ) {
        (Ok(lista), Ok(info)) => Some((lista, info)),
        _ => None,
    };
    let _objetos = if let Some((lista, info)) = cosmic {
        state.info = Some(info);
        println!("BACKEND cosmic-toplevel-info");
        (Some(lista), None)
    } else {
        let gerente: WManager = globals.bind(&qh, 1..=3, ())?;
        println!("BACKEND wlr-foreign-toplevel");
        (None, Some(gerente))
    };

    loop {
        queue.blocking_dispatch(&mut state)?;
    }
}
