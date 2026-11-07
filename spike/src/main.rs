//! Kit de validação: prova, num Pop!_OS real, que a arquitetura do Super Cola se sustenta.
mod dc;
mod focus;
mod foto;
mod panel;
mod payload;
mod pointer;
mod probe;
mod target;

pub type R<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const AJUDA: &str = "\
uso: supercola-spike <comando> [opções]

  probe                     lista os protocolos Wayland disponíveis
  watch                     monitora o clipboard (imprime só tamanho e hash)
  focus                     imprime o app em foco e cada mudança de foco
  target                    abre a janela-alvo, que mede o que foi colado
  panel [opções]            abre o painel de prova
  serve [--size N]          oferece um texto via data-control e sai se perder a seleção
  paste [--shift]           injeta Ctrl+V (ou Ctrl+Shift+V) no app em foco
  tecla NOME                aperta uma tecla pelo nome xkb, como Return (só no COSMIC)
  click X Y                 clica por ponteiro virtual (para testes em sway)
  hover X Y MS              deixa um ponteiro virtual parado por MS milissegundos
  foto PASTA                captura cada saída em um arquivo PPM

opções de panel:
  --auto          confirma sozinho ao receber o foco, sem precisar de Enter
  --dc            copia via data-control (o padrão é GDK)
  --full          superfície de tela cheia, que captura o clique fora
  --paste         cola sozinho depois de copiar
  --shift         cola com Ctrl+Shift+V
  --size N        tamanho do conteúdo em bytes (0 = texto curto)
  --delay MS      espera antes de injetar (padrão 150)
  --hold MS       mantém o processo vivo depois de copiar (padrão 3000)
";

fn numero(args: &[String], nome: &str, padrao: u64) -> R<u64> {
    match args.iter().position(|a| a == nome) {
        None => Ok(padrao),
        Some(i) => {
            let valor = args
                .get(i + 1)
                .ok_or_else(|| format!("{nome} precisa de um valor"))?;
            Ok(valor
                .parse()
                .map_err(|_| format!("{nome}: número inválido: {valor}"))?)
        }
    }
}

fn run() -> R {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let tem = |nome: &str| args.iter().any(|a| a == nome);
    match args.first().map(String::as_str) {
        Some("probe") => probe::run(),
        Some("watch") => dc::watch(),
        Some("focus") => focus::run(),
        Some("target") => target::run(),
        Some("panel") => panel::run(panel::Opts {
            auto: tem("--auto"),
            dc: tem("--dc"),
            full: tem("--full"),
            paste: tem("--paste"),
            shift: tem("--shift"),
            size: numero(&args, "--size", 0)? as usize,
            delay_ms: numero(&args, "--delay", 150)?,
            hold_ms: numero(&args, "--hold", 3000)?,
        }),
        Some("serve") => {
            let texto = payload::make(numero(&args, "--size", 0)? as usize);
            println!(
                "SERVE len={} hash={:016x}",
                texto.len(),
                payload::hash(texto.as_bytes())
            );
            dc::serve(std::sync::Arc::new(texto))
        }
        Some("foto") => foto::run(args.get(1).ok_or("uso: foto PASTA")?),
        Some("paste") => supercola_wayland::keyboard::paste(tem("--shift")),
        Some("tecla") => supercola_wayland::keyboard::tap(args.get(1).ok_or("uso: tecla NOME")?),
        Some("click") => {
            let coord = |i: usize| -> R<u32> { Ok(args.get(i).ok_or("uso: click X Y")?.parse()?) };
            pointer::click(coord(1)?, coord(2)?)
        }
        Some("hover") => {
            let coord =
                |i: usize| -> R<u32> { Ok(args.get(i).ok_or("uso: hover X Y MS")?.parse()?) };
            pointer::hover(coord(1)?, coord(2)?, u64::from(coord(3)?))
        }
        _ => {
            print!("{AJUDA}");
            Ok(())
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("ERRO: {e}");
        std::process::exit(1);
    }
}
