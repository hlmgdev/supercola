//! Socket Unix do daemon. Fica em `$XDG_RUNTIME_DIR`, com permissão 0600.
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc;
use std::time::Duration;

use supercola_core::ipc::{socket_path, Command};

pub struct Pedido {
    pub comando: Command,
    pub resposta: mpsc::Sender<String>,
}

fn atender(sock: UnixStream, tx: &async_channel::Sender<Pedido>) {
    let _ = sock.set_read_timeout(Some(Duration::from_secs(2)));
    let mut linha = String::new();
    if BufReader::new(&sock).take_line(&mut linha).is_err() {
        return;
    }
    let Some(comando) = Command::parse(&linha) else {
        let _ = (&sock).write_all(b"comando desconhecido\n");
        return;
    };
    let (resposta, rx) = mpsc::channel();
    if tx.send_blocking(Pedido { comando, resposta }).is_ok() {
        if let Ok(texto) = rx.recv_timeout(Duration::from_secs(3)) {
            let _ = (&sock).write_all(texto.as_bytes());
        }
    }
}

trait LinhaLimitada {
    fn take_line(self, linha: &mut String) -> std::io::Result<usize>;
}

impl<R: std::io::Read> LinhaLimitada for BufReader<R> {
    fn take_line(self, linha: &mut String) -> std::io::Result<usize> {
        use std::io::Read;
        BufReader::new(self.take(256)).read_line(linha)
    }
}

/// Falha se outro daemon já estiver rodando.
pub fn listen() -> Result<async_channel::Receiver<Pedido>, String> {
    let caminho = socket_path();
    if UnixStream::connect(&caminho).is_ok() {
        return Err("o supercola já está rodando".into());
    }
    let _ = fs::remove_file(&caminho);
    let ouvinte = UnixListener::bind(&caminho).map_err(|e| e.to_string())?;
    fs::set_permissions(&caminho, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    let (tx, rx) = async_channel::unbounded();
    std::thread::Builder::new()
        .name("ipc".into())
        .spawn(move || {
            for sock in ouvinte.incoming().flatten() {
                atender(sock, &tx);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(rx)
}
