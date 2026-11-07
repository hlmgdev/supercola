//! Cliente mínimo chamado pelo atalho. Sem dependências, para abrir instantaneamente.
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

const AJUDA: &str = "\
uso: supercola-toggle [comando]

  (nenhum)    abre ou fecha o histórico
  home        abre ou fecha a aba inicial
  emoji       abre ou fecha a aba Emoji
  gif         abre ou fecha a aba GIF
  kaomoji     abre ou fecha a aba Kaomoji
  symbols     abre ou fecha a aba Símbolos
  settings    abre as configurações
  hide        fecha o painel
  status      mostra quantos itens há, sem exibir conteúdo
  quit        encerra o daemon
";

fn socket() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|v| !v.is_empty())
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("supercola.sock")
}

fn conectar() -> Option<UnixStream> {
    UnixStream::connect(socket()).ok()
}

/// O autostart pode ter falhado: o atalho ainda precisa funcionar.
fn iniciar_daemon() -> Option<UnixStream> {
    Command::new("supercola")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(50));
        if let Some(s) = conectar() {
            return Some(s);
        }
    }
    None
}

fn main() {
    let arg = std::env::args().nth(1);
    let linha = match arg.as_deref() {
        None => "toggle\n".to_string(),
        Some("-h" | "--help") => {
            print!("{AJUDA}");
            return;
        }
        Some(aba @ ("home" | "emoji" | "gif" | "kaomoji" | "symbols" | "history")) => {
            format!("toggle {aba}\n")
        }
        Some(cmd @ ("settings" | "hide" | "status" | "quit")) => format!("{cmd}\n"),
        Some(outro) => {
            eprintln!("comando desconhecido: {outro}\n\n{AJUDA}");
            std::process::exit(2);
        }
    };
    let quer_daemon = !matches!(arg.as_deref(), Some("quit" | "status" | "hide"));
    let conexao = conectar().or_else(|| if quer_daemon { iniciar_daemon() } else { None });
    let Some(mut sock) = conexao else {
        eprintln!("supercola: daemon indisponível");
        std::process::exit(1);
    };
    let _ = sock.set_read_timeout(Some(Duration::from_secs(5)));
    if sock.write_all(linha.as_bytes()).is_err() {
        std::process::exit(1);
    }
    let mut resposta = String::new();
    let _ = sock.read_to_string(&mut resposta);
    print!("{resposta}");
}
