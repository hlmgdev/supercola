//! Captura cada saída para um PPM, para ver o painel real no COSMIC.
//!
//! O `cosmic-screenshot` fica esperando o portal fora do modo interativo, e o `grim` do
//! Noble não conhece `ext-image-copy-capture`.
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::os::fd::AsFd;
use std::os::unix::fs::FileExt;
use std::path::Path;

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_shm::{self, WlShm};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::{delegate_noop, Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_image_capture_source_v1::ExtImageCaptureSourceV1 as Fonte,
    ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1 as Fontes,
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::{self as quadro, ExtImageCopyCaptureFrameV1 as Quadro},
    ext_image_copy_capture_manager_v1::{ExtImageCopyCaptureManagerV1 as Capturas, Options},
    ext_image_copy_capture_session_v1::{self as sessao, ExtImageCopyCaptureSessionV1 as Sessao},
};

#[derive(Default)]
struct State {
    largura: u32,
    altura: u32,
    formatos: Vec<wl_shm::Format>,
    sessao_pronta: bool,
    capturado: bool,
    falha: Option<String>,
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

impl Dispatch<Sessao, ()> for State {
    fn event(
        state: &mut Self,
        _: &Sessao,
        event: sessao::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            sessao::Event::BufferSize { width, height } => {
                state.largura = width;
                state.altura = height;
            }
            sessao::Event::ShmFormat {
                format: WEnum::Value(f),
            } => state.formatos.push(f),
            sessao::Event::Done => state.sessao_pronta = true,
            sessao::Event::Stopped => state.falha = Some("sessão encerrada".into()),
            _ => {}
        }
    }
}

impl Dispatch<Quadro, ()> for State {
    fn event(
        state: &mut Self,
        _: &Quadro,
        event: quadro::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            quadro::Event::Ready => state.capturado = true,
            quadro::Event::Failed { reason } => state.falha = Some(format!("{reason:?}")),
            _ => {}
        }
    }
}

delegate_noop!(State: ignore WlShm);
delegate_noop!(State: ignore WlShmPool);
delegate_noop!(State: ignore WlBuffer);
delegate_noop!(State: ignore WlOutput);
delegate_noop!(State: Fonte);
delegate_noop!(State: Fontes);
delegate_noop!(State: Capturas);

/// Posição do vermelho e do azul no pixel de 4 bytes, em little-endian.
fn canais(formato: wl_shm::Format) -> Option<(usize, usize)> {
    match formato {
        wl_shm::Format::Argb8888 | wl_shm::Format::Xrgb8888 => Some((2, 0)),
        wl_shm::Format::Abgr8888 | wl_shm::Format::Xbgr8888 => Some((0, 2)),
        _ => None,
    }
}

pub fn run(pasta: &str) -> crate::R {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&conn)?;
    let qh = queue.handle();
    let shm: WlShm = globals.bind(&qh, 1..=1, ())?;
    let fontes: Fontes = globals.bind(&qh, 1..=1, ())?;
    let capturas: Capturas = globals.bind(&qh, 1..=1, ())?;
    let saidas: Vec<WlOutput> = globals
        .contents()
        .clone_list()
        .iter()
        .filter(|g| g.interface == "wl_output")
        .map(|g| globals.registry().bind(g.name, 1, &qh, ()))
        .collect();

    for (i, saida) in saidas.iter().enumerate() {
        let mut state = State::default();
        let fonte = fontes.create_source(saida, &qh, ());
        let sessao = capturas.create_session(&fonte, Options::empty(), &qh, ());
        while !state.sessao_pronta && state.falha.is_none() {
            queue.blocking_dispatch(&mut state)?;
        }
        let formato = state
            .formatos
            .iter()
            .copied()
            .find(|f| canais(*f).is_some())
            .ok_or("sem formato de 32 bits")?;
        let (vermelho, azul) = canais(formato).ok_or("formato inesperado")?;
        let (largura, altura) = (state.largura, state.altura);
        let passo = largura * 4;
        let tamanho = passo * altura;

        let caminho = std::env::temp_dir().join(format!("supercola-foto-{}", std::process::id()));
        let arquivo: File = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&caminho)?;
        std::fs::remove_file(&caminho)?;
        arquivo.set_len(u64::from(tamanho))?;
        let pool = shm.create_pool(arquivo.as_fd(), tamanho as i32, &qh, ());
        let buffer = pool.create_buffer(
            0,
            largura as i32,
            altura as i32,
            passo as i32,
            formato,
            &qh,
            (),
        );
        let quadro = sessao.create_frame(&qh, ());
        quadro.attach_buffer(&buffer);
        quadro.damage_buffer(0, 0, largura as i32, altura as i32);
        quadro.capture();
        while !state.capturado && state.falha.is_none() {
            queue.blocking_dispatch(&mut state)?;
        }
        if let Some(falha) = state.falha {
            return Err(format!("captura {i}: {falha}").into());
        }

        let mut pixels = vec![0; tamanho as usize];
        arquivo.read_exact_at(&mut pixels, 0)?;
        let destino = Path::new(pasta).join(format!("saida-{i}.ppm"));
        let mut ppm = BufWriter::new(File::create(&destino)?);
        write!(ppm, "P6\n{largura} {altura}\n255\n")?;
        for px in pixels.as_chunks::<4>().0 {
            ppm.write_all(&[px[vermelho], px[1], px[azul]])?;
        }
        ppm.flush()?;
        println!("{} {largura}x{altura}", destino.display());
    }
    Ok(())
}
