//! Aba de GIF. É a única parte do Super Cola que acessa a rede.
use std::cell::{Cell, RefCell};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use gtk4::gdk_pixbuf::PixbufAnimation;
use gtk4::prelude::*;
use gtk4::{
    gdk, glib, Align, Box as GtkBox, Button, FlowBox, FlowBoxChild, Label, Orientation, Picture,
    PolicyType, ScrolledWindow, SelectionMode, Stack,
};
use supercola_core::config::{Config, GifProvider};
use supercola_core::gif::{parse, request_url, GifItem};
use supercola_core::i18n::Lang;
use supercola_core::model::{Flavor, MIME_GIF, MIME_TEXT, MIME_URIS};
use supercola_core::xdg;

use crate::app::App;
use crate::grid::botao_grupo;
use crate::history::MAX_LARGURA_TEXTO;
use crate::panel::icone_aba;

const MAX_GIF: u64 = 4 * 1024 * 1024;
const MAX_JSON: u64 = 2 * 1024 * 1024;
const COLUNAS: u32 = 3;
const LARGURA_CELULA: i32 = 104;
const ALTURA_CELULA: i32 = 90;

/// Em testes, troca o servidor por um local.
fn endereco(url: &str) -> String {
    let Ok(base) = std::env::var("SUPERCOLA_GIF_TEST_BASE") else {
        return url.to_string();
    };
    let sem_esquema = url.strip_prefix("https://").unwrap_or(url);
    let caminho = sem_esquema.split_once('/').map_or("", |(_, c)| c);
    format!("{base}/{caminho}")
}

/// A mensagem de erro não leva a URL, que contém a chave da API.
fn baixar(url: &str, limite: u64) -> Result<Vec<u8>, String> {
    let resposta = ureq::get(&endereco(url))
        .timeout(Duration::from_secs(10))
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(c, _) => format!("HTTP {c}"),
            ureq::Error::Transport(_) => "falha de rede".to_string(),
        })?;
    let mut dados = Vec::new();
    resposta
        .into_reader()
        .take(limite + 1)
        .read_to_end(&mut dados)
        .map_err(|_| "falha de rede".to_string())?;
    if dados.len() as u64 > limite {
        return Err("resposta grande demais".into());
    }
    Ok(dados)
}

fn caminho_cache(url: &str) -> PathBuf {
    let hash = url.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    xdg::cache_dir()
        .join("gif")
        .join(format!("{hash:016x}.gif"))
}

fn baixar_em_cache(url: &str) -> Result<PathBuf, String> {
    let destino = caminho_cache(url);
    if destino.exists() {
        return Ok(destino);
    }
    let dados = baixar(url, MAX_GIF)?;
    if let Some(pai) = destino.parent() {
        fs::create_dir_all(pai).map_err(|e| e.to_string())?;
    }
    fs::write(&destino, dados).map_err(|e| e.to_string())?;
    Ok(destino)
}

/// Roda `f` em outra thread e entrega o resultado na thread do GTK.
fn em_segundo_plano<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
    feito: impl FnOnce(T) + 'static,
) {
    let (tx, rx) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let _ = tx.send_blocking(f());
    });
    glib::spawn_future_local(async move {
        if let Ok(r) = rx.recv().await {
            feito(r);
        }
    });
}

fn animar(figura: &Picture, animacao: PixbufAnimation) {
    let iter = animacao.iter(Some(SystemTime::now()));
    figura.set_paintable(Some(&gdk::Texture::for_pixbuf(&iter.pixbuf())));
    let fraca = figura.downgrade();
    glib::timeout_add_local(Duration::from_millis(66), move || {
        let Some(figura) = fraca.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if figura.is_mapped() && iter.advance(SystemTime::now()) {
            figura.set_paintable(Some(&gdk::Texture::for_pixbuf(&iter.pixbuf())));
        }
        glib::ControlFlow::Continue
    });
}

pub struct GifTab {
    pub root: GtkBox,
    /// Categoria única, que o painel mostra no lugar das abas.
    pub barra: GtkBox,
    titulo: Label,
    fornecedor: Label,
    conteudo: Stack,
    grade: FlowBox,
    aviso: Label,
    geracao: Rc<Cell<u64>>,
    itens: Rc<RefCell<Vec<GifItem>>>,
}

impl GifTab {
    pub fn new(app: &Rc<App>) -> Self {
        let root = GtkBox::new(Orientation::Vertical, 0);
        let titulo = Label::new(Some(app.tr("gif_trending")));
        titulo.add_css_class("sc-title");
        titulo.set_xalign(0.0);
        titulo.set_margin_top(14);
        titulo.set_margin_start(16);
        root.append(&titulo);
        let fornecedor = Label::new(None);
        fornecedor.add_css_class("sc-link-texto");
        fornecedor.set_xalign(0.0);
        fornecedor.set_margin_start(16);
        fornecedor.set_margin_bottom(6);
        root.append(&fornecedor);
        let conteudo = Stack::new();
        conteudo.set_vexpand(true);
        root.append(&conteudo);

        let barra = GtkBox::new(Orientation::Horizontal, 0);
        let em_alta = botao_grupo(
            &icone_aba("supercola-trending-symbolic"),
            app.tr("gif_trending"),
        );
        em_alta.set_active(true);
        em_alta.set_can_target(false);
        barra.append(&em_alta);

        let grade = FlowBox::new();
        grade.add_css_class("sc-gifs");
        grade.set_selection_mode(SelectionMode::None);
        grade.set_min_children_per_line(COLUNAS);
        grade.set_max_children_per_line(COLUNAS);
        grade.set_homogeneous(true);
        grade.set_activate_on_single_click(true);
        let itens: Rc<RefCell<Vec<GifItem>>> = Rc::default();
        let (a, lista) = (app.clone(), itens.clone());
        grade.connect_child_activated(move |_, filho| {
            let item = lista.borrow().get(filho.index() as usize).cloned();
            if let Some(item) = item {
                colar(&a, &item);
            }
        });
        grade.set_valign(Align::Start);
        grade.set_margin_start(14);
        grade.set_margin_end(14);
        let rolagem = ScrolledWindow::new();
        rolagem.set_hscrollbar_policy(PolicyType::Never);
        rolagem.set_child(Some(&grade));
        conteudo.add_named(&rolagem, Some("grade"));

        let ativar = GtkBox::new(Orientation::Vertical, 8);
        ativar.set_valign(Align::Center);
        ativar.set_margin_start(32);
        ativar.set_margin_end(32);
        let t = Label::new(Some(app.tr("gif_setup_title")));
        t.add_css_class("sc-empty-title");
        ativar.append(&t);
        let c = Label::new(Some(app.tr("gif_setup_body")));
        c.add_css_class("sc-secondary");
        c.set_wrap(true);
        c.set_max_width_chars(MAX_LARGURA_TEXTO);
        c.set_justify(gtk4::Justification::Center);
        ativar.append(&c);
        let b = Button::with_label(app.tr("open_settings"));
        b.add_css_class("sc-accent-btn");
        b.set_halign(Align::Center);
        b.set_margin_top(8);
        let a = app.clone();
        b.connect_clicked(move |_| {
            a.esconder();
            crate::settings::abrir(&a);
        });
        ativar.append(&b);
        conteudo.add_named(&ativar, Some("ativar"));

        let aviso = Label::new(None);
        aviso.add_css_class("sc-secondary");
        aviso.set_valign(Align::Center);
        aviso.set_wrap(true);
        aviso.set_max_width_chars(MAX_LARGURA_TEXTO);
        conteudo.add_named(&aviso, Some("aviso"));

        Self {
            root,
            barra,
            titulo,
            fornecedor,
            conteudo,
            grade,
            aviso,
            geracao: Rc::new(Cell::new(0)),
            itens,
        }
    }

    pub fn focar(&self) -> bool {
        self.grade.child_at_index(0).is_some_and(|c| c.grab_focus())
    }

    pub fn ativar_selecionado(&self, app: &Rc<App>) {
        let foco = self
            .grade
            .root()
            .and_then(|r| gtk4::prelude::RootExt::focus(&r))
            .and_then(|w| w.downcast::<FlowBoxChild>().ok());
        let item = foco.and_then(|f| self.itens.borrow().get(f.index() as usize).cloned());
        if let Some(item) = item {
            colar(app, &item);
        }
    }

    fn avisar(&self, texto: &str) {
        self.aviso.set_text(texto);
        self.conteudo.set_visible_child_name("aviso");
    }

    pub fn buscar(&self, app: &Rc<App>, texto: &str) {
        let cfg: Config = app.cfg.borrow().clone();
        self.titulo.set_text(app.tr(if texto.trim().is_empty() {
            "gif_trending"
        } else {
            "search_results"
        }));
        if cfg.gif_api_key.trim().is_empty() {
            self.fornecedor.set_text("");
            self.conteudo.set_visible_child_name("ativar");
            return;
        }
        let servico = match cfg.gif_provider {
            GifProvider::Klipy => "KLIPY",
            GifProvider::Giphy => "GIPHY",
        };
        self.fornecedor
            .set_text(&format!("{} {servico}", app.tr("gif_powered")));
        // Só a resposta da busca mais nova interessa.
        let geracao = self.geracao.get() + 1;
        self.geracao.set(geracao);
        let locale = if app.lang.get() == Lang::Pt {
            "pt_BR"
        } else {
            "en_US"
        };
        let url = request_url(cfg.gif_provider, &cfg.gif_api_key, Some(texto), locale);
        let provedor = cfg.gif_provider;

        let (grade, conteudo, aviso, atual, a, lista) = (
            self.grade.clone(),
            self.conteudo.clone(),
            self.aviso.clone(),
            self.geracao.clone(),
            app.clone(),
            self.itens.clone(),
        );
        em_segundo_plano(
            move || baixar(&url, MAX_JSON).and_then(|c| parse(provedor, &c)),
            move |r: Result<Vec<GifItem>, String>| {
                if atual.get() != geracao {
                    return;
                }
                match r {
                    Err(e) => {
                        eprintln!("supercola: busca de GIF falhou: {e}");
                        aviso.set_text(a.tr("gif_error"));
                        conteudo.set_visible_child_name("aviso");
                    }
                    Ok(itens) if itens.is_empty() => {
                        aviso.set_text(a.tr("no_results"));
                        conteudo.set_visible_child_name("aviso");
                    }
                    Ok(itens) => {
                        grade.remove_all();
                        for item in &itens {
                            grade.append(&celula(item));
                        }
                        *lista.borrow_mut() = itens;
                        conteudo.set_visible_child_name("grade");
                    }
                }
            },
        );
        if self.grade.first_child().is_none() {
            self.avisar("…");
        }
    }
}

fn celula(item: &GifItem) -> FlowBoxChild {
    let figura = Picture::new();
    figura.add_css_class("sc-gif");
    figura.set_size_request(LARGURA_CELULA, ALTURA_CELULA);
    figura.set_content_fit(gtk4::ContentFit::Cover);
    figura.set_can_shrink(true);
    figura.set_alternative_text(Some(&item.title));

    let previa = item.preview_url.clone();
    let f = figura.clone();
    em_segundo_plano(
        move || baixar_em_cache(&previa),
        move |r| {
            if let Ok(caminho) = r {
                if let Ok(animacao) = PixbufAnimation::from_file(&caminho) {
                    animar(&f, animacao);
                }
            }
        },
    );

    let filho = FlowBoxChild::new();
    filho.set_child(Some(&figura));
    filho
}

/// Baixa antes de fechar o painel: a cópia comum exige o painel em foco.
fn colar(app: &Rc<App>, item: &GifItem) {
    let url = item.full_url.clone();
    let u = url.clone();
    let a = app.clone();
    em_segundo_plano(
        move || baixar_em_cache(&u).and_then(|p| fs::read(p).map_err(|e| e.to_string())),
        move |r: Result<Vec<u8>, String>| match r {
            Ok(dados) => a.colar(
                vec![
                    Flavor::new(MIME_GIF, dados),
                    Flavor::new(MIME_URIS, format!("{url}\r\n").into_bytes()),
                    Flavor::new(MIME_TEXT, url.into_bytes()),
                ],
                true,
            ),
            Err(e) => eprintln!("supercola: não consegui baixar o GIF: {e}"),
        },
    );
}
