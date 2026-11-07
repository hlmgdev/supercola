//! Monitora e grava o clipboard por data-control: `ext` de preferência, `wlr` como reserva.
use std::io::Read;
use std::os::fd::{AsRawFd, RawFd};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use supercola_core::history::MAX_ITEM_BYTES;
use supercola_core::model::Flavor;

/// O que acontece no clipboard, entregue fora da thread do GTK.
#[derive(Debug)]
pub enum Event {
    /// Novo conteúdo, com os formatos que interessam.
    Selection(Vec<Flavor>),
    /// Conteúdo marcado como senha: foi ignorado sem ser lido.
    Secret,
    /// O dono da seleção não respondeu, ou o item passa de 4 MB.
    Skipped,
}

pub type Sink = Arc<dyn Fn(Event) + Send + Sync>;

/// Par formato anunciado e dados, já com os apelidos de texto.
type Oferta = Vec<(String, Arc<Vec<u8>>)>;

/// Canal para pedir escritas à thread do Wayland.
pub struct Handle {
    tx: mpsc::Sender<Oferta>,
    wake: std::io::PipeWriter,
    pub backend: &'static str,
}

impl Handle {
    pub fn set(&self, offered: Vec<(String, Arc<Vec<u8>>)>) {
        use std::io::Write;
        if self.tx.send(offered).is_ok() {
            let _ = (&self.wake).write_all(&[1]);
        }
    }
}

/// Lê tudo, com limite de tamanho e de tempo: um app travado não pode segurar a thread.
fn ler(mut leitor: std::io::PipeReader) -> Option<Vec<u8>> {
    const TEMPO: Duration = Duration::from_secs(2);
    let fd: RawFd = leitor.as_raw_fd();
    let fim = Instant::now() + TEMPO;
    let mut dados = Vec::new();
    let mut bloco = [0u8; 65536];
    loop {
        let resta = fim.checked_duration_since(Instant::now())?;
        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: `pfd` vive durante a chamada e `fd` pertence a `leitor`.
        let pronto = unsafe { libc::poll(&mut pfd, 1, resta.as_millis() as libc::c_int) };
        if pronto < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return None;
        }
        if pronto == 0 {
            return None;
        }
        match leitor.read(&mut bloco) {
            Ok(0) => return Some(dados),
            Ok(n) => {
                dados.extend_from_slice(&bloco[..n]);
                if dados.len() > MAX_ITEM_BYTES {
                    return None;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
}

fn ler_tudo(canos: Vec<(String, std::io::PipeReader)>, sink: Sink) {
    let mut sabores = Vec::new();
    for (mime, cano) in canos {
        let Some(dados) = ler(cano) else {
            sink(Event::Skipped);
            return;
        };
        if !dados.is_empty() {
            sabores.push(Flavor::new(mime, dados));
        }
    }
    if !sabores.is_empty() {
        sink(Event::Selection(sabores));
    }
}

macro_rules! backend {
    ($modulo:ident, $nome:literal, $versao:expr, { $($imports:tt)* }) => {
        mod $modulo {
            use super::*;
            $($imports)*
            const BACKEND: &str = $nome;
            const VERSAO: u32 = $versao;
            include!("dc_impl.rs");
        }
    };
}

backend!(ext, "ext-data-control", 1, {
    use wayland_protocols::ext::data_control::v1::client::{
        ext_data_control_device_v1::{self as dev, ExtDataControlDeviceV1 as Device},
        ext_data_control_manager_v1::ExtDataControlManagerV1 as Manager,
        ext_data_control_offer_v1::{self as ofr, ExtDataControlOfferV1 as Offer},
        ext_data_control_source_v1::{self as src, ExtDataControlSourceV1 as Source},
    };
});

backend!(wlr, "wlr-data-control", 1, {
    use wayland_protocols_wlr::data_control::v1::client::{
        zwlr_data_control_device_v1::{self as dev, ZwlrDataControlDeviceV1 as Device},
        zwlr_data_control_manager_v1::ZwlrDataControlManagerV1 as Manager,
        zwlr_data_control_offer_v1::{self as ofr, ZwlrDataControlOfferV1 as Offer},
        zwlr_data_control_source_v1::{self as src, ZwlrDataControlSourceV1 as Source},
    };
});

pub fn spawn(sink: Sink) -> crate::R<Handle> {
    ext::start(sink.clone()).or_else(|_| wlr::start(sink))
}
