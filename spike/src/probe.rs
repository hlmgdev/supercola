//! Lista os globais Wayland de que a arquitetura precisa.
use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};

const INTERESSE: &[(&str, &str)] = &[
    (
        "ext_data_control_manager_v1",
        "ler/escrever clipboard (preferido)",
    ),
    (
        "zwlr_data_control_manager_v1",
        "ler/escrever clipboard (reserva)",
    ),
    ("zwlr_layer_shell_v1", "painel flutuante"),
    ("zwp_virtual_keyboard_manager_v1", "injetar Ctrl+V"),
    ("ext_foreign_toplevel_list_v1", "listar janelas"),
    ("zcosmic_toplevel_info_v1", "descobrir a janela em foco"),
    ("ext_background_effect_manager_v1", "desfoque"),
    ("wl_data_device_manager", "clipboard comum"),
];

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

pub fn run() -> crate::R {
    let conn = Connection::connect_to_env()?;
    let mut queue = conn.new_event_queue();
    conn.display().get_registry(&queue.handle(), ());
    let mut globais = Globais::default();
    queue.roundtrip(&mut globais)?;

    for (nome, papel) in INTERESSE {
        match globais.0.iter().find(|(i, _)| i == nome) {
            Some((_, v)) => println!("OK      {nome} v{v}  # {papel}"),
            None => println!("AUSENTE {nome}  # {papel}"),
        }
    }
    println!("--- todos os globais");
    for (i, v) in &globais.0 {
        println!("{i} v{v}");
    }
    Ok(())
}
