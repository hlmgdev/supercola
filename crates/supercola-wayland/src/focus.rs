//! Acompanha o app em foco: protocolo do COSMIC, com o `wlr` como reserva.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

/// Último app em foco. Um painel layer-shell não é janela, então não o substitui.
#[derive(Clone, Default)]
pub struct Focus(Arc<Mutex<Option<String>>>);

impl Focus {
    pub fn current(&self) -> Option<String> {
        self.0.lock().ok()?.clone()
    }
    fn set(&self, app: String) {
        if let Ok(mut f) = self.0.lock() {
            *f = Some(app);
        }
    }
}

#[derive(Default)]
struct Janela {
    app_id: String,
    ativa: bool,
}

struct State {
    janelas: HashMap<ObjectId, Janela>,
    cosmic_para_foreign: HashMap<ObjectId, ObjectId>,
    info: Option<Info>,
    foco: Focus,
}

impl State {
    fn atualizar(&mut self) {
        if let Some(j) = self
            .janelas
            .values()
            .find(|j| j.ativa && !j.app_id.is_empty())
        {
            self.foco.set(j.app_id.clone());
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
            }
            fhandle::Event::Done => state.atualizar(),
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
            state.atualizar();
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
            }
            whandle::Event::Done => state.atualizar(),
            _ => {}
        }
    }
}

/// Sem protocolo disponível devolve erro, e o Super Cola segue sem detectar terminais.
pub fn spawn() -> crate::R<(Focus, &'static str)> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&conn)?;
    let qh = queue.handle();
    let foco = Focus::default();
    let mut state = State {
        janelas: HashMap::new(),
        cosmic_para_foreign: HashMap::new(),
        info: None,
        foco: foco.clone(),
    };

    let cosmic = match (
        globals.bind::<FList, _, _>(&qh, 1..=1, ()),
        globals.bind::<Info, _, _>(&qh, 2..=3, ()),
    ) {
        (Ok(lista), Ok(info)) => Some((lista, info)),
        _ => None,
    };
    let (objetos, backend) = if let Some((lista, info)) = cosmic {
        state.info = Some(info);
        ((Some(lista), None), "cosmic-toplevel-info")
    } else {
        let gerente: WManager = globals.bind(&qh, 1..=3, ())?;
        ((None, Some(gerente)), "wlr-foreign-toplevel")
    };

    std::thread::Builder::new()
        .name("foco".into())
        .spawn(move || {
            let _objetos = objetos;
            while queue.blocking_dispatch(&mut state).is_ok() {}
        })?;
    Ok((foco, backend))
}
