//! Aba do histórico.
use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    gdk, glib, pango, Align, Box as GtkBox, Button, Label, ListBox, ListBoxRow, Orientation,
    Picture, PolicyType, ScrolledWindow, SelectionMode, Stack,
};
use supercola_core::model::{Clip, Kind, MIME_PNG};

use crate::app::App;
use crate::panel::icone;

const MAX_CARACTERES: usize = 600;
/// O item tem altura fixa, como no Windows, e estas linhas cabem nele.
const LINHAS: i32 = 3;
const ALTURA_MINIATURA: i32 = 64;
const LARGURA_MINIATURA: i32 = 220;
/// Rótulos com quebra de linha pedem a largura do texto inteiro sem este limite.
pub const MAX_LARGURA_TEXTO: i32 = 34;

pub struct HistoryTab {
    pub root: GtkBox,
    cabecalho: GtkBox,
    conteudo: Stack,
    lista: ListBox,
    limpar: Button,
    /// As ações atrás do `⋯` de cada linha: só uma fica aberta por vez.
    acoes_abertas: Rc<RefCell<Vec<GtkBox>>>,
}

fn id_da_linha(linha: &ListBoxRow) -> Option<u64> {
    linha.widget_name().parse().ok()
}

fn vazio(app: &Rc<App>, titulo: &str, corpo: &str, botao: Option<&str>) -> GtkBox {
    let caixa = GtkBox::new(Orientation::Vertical, 8);
    caixa.set_valign(Align::Center);
    caixa.set_margin_start(32);
    caixa.set_margin_end(32);
    let t = Label::new(Some(titulo));
    t.add_css_class("sc-empty-title");
    t.set_wrap(true);
    t.set_max_width_chars(MAX_LARGURA_TEXTO);
    t.set_justify(gtk4::Justification::Center);
    caixa.append(&t);
    let c = Label::new(Some(corpo));
    c.add_css_class("sc-secondary");
    c.set_wrap(true);
    c.set_max_width_chars(MAX_LARGURA_TEXTO);
    c.set_justify(gtk4::Justification::Center);
    caixa.append(&c);
    if let Some(texto) = botao {
        let b = Button::with_label(texto);
        b.add_css_class("sc-accent-btn");
        b.set_halign(Align::Center);
        b.set_margin_top(8);
        let a = app.clone();
        b.connect_clicked(move |_| a.alterna_historico(true));
        caixa.append(&b);
    }
    caixa
}

fn resumo(texto: &str) -> String {
    let inicio = texto.trim_start();
    let mut s: String = inicio.chars().take(MAX_CARACTERES).collect();
    if inicio.chars().count() > MAX_CARACTERES {
        s.push('…');
    }
    s
}

fn textura(app: &App, clip: &Clip) -> Option<gdk::Texture> {
    if let Some(t) = app.texturas.borrow().get(&clip.id) {
        return Some(t.clone());
    }
    let dados = clip.flavor(MIME_PNG)?;
    let t = gdk::Texture::from_bytes(&glib::Bytes::from(dados.data.as_slice())).ok()?;
    app.texturas.borrow_mut().insert(clip.id, t.clone());
    Some(t)
}

impl HistoryTab {
    pub fn new(app: &Rc<App>) -> Self {
        let root = GtkBox::new(Orientation::Vertical, 0);

        let topo = GtkBox::new(Orientation::Horizontal, 0);
        topo.set_margin_top(22);
        topo.set_margin_start(16);
        topo.set_margin_end(16);
        topo.set_margin_bottom(10);
        let titulo = Label::new(Some(app.tr("clipboard_history")));
        titulo.add_css_class("sc-title");
        titulo.set_hexpand(true);
        titulo.set_xalign(0.0);
        titulo.set_ellipsize(pango::EllipsizeMode::End);
        topo.append(&titulo);
        let limpar = Button::with_label(app.tr("clear_all"));
        limpar.add_css_class("sc-botao");
        limpar.set_focus_on_click(false);
        topo.append(&limpar);
        root.append(&topo);

        let lista = ListBox::new();
        lista.add_css_class("sc-list");
        lista.set_selection_mode(SelectionMode::Single);
        lista.set_activate_on_single_click(true);
        lista.set_margin_start(16);
        lista.set_margin_end(16);
        lista.set_margin_bottom(4);
        let a = app.clone();
        lista.connect_row_activated(move |_, linha| {
            if let Some(id) = id_da_linha(linha) {
                a.colar_item(id, false);
            }
        });

        let rolagem = ScrolledWindow::new();
        rolagem.set_hscrollbar_policy(PolicyType::Never);
        rolagem.set_vexpand(true);
        rolagem.set_child(Some(&lista));

        let conteudo = Stack::new();
        conteudo.set_vexpand(true);
        conteudo.add_named(&rolagem, Some("lista"));
        conteudo.add_named(
            &vazio(app, app.tr("empty_title"), app.tr("empty_body"), None),
            Some("vazio"),
        );
        conteudo.add_named(
            &vazio(
                app,
                app.tr("off_title"),
                app.tr("off_body"),
                Some(app.tr("turn_on")),
            ),
            Some("desligado"),
        );
        conteudo.add_named(
            &vazio(app, app.tr("no_results"), "", None),
            Some("sem-resultado"),
        );
        root.append(&conteudo);

        let a = app.clone();
        limpar.connect_clicked(move |_| {
            a.history.borrow_mut().clear();
            a.texturas.borrow_mut().clear();
            a.salvar_fixados();
            a.ao_mudar_historico();
        });

        Self {
            root,
            cabecalho: topo,
            conteudo,
            lista,
            limpar,
            acoes_abertas: Rc::default(),
        }
    }

    pub fn mostrar_cabecalho(&self, visivel: bool) {
        self.cabecalho.set_visible(visivel);
    }

    /// Ações que aparecem ao lado do item quando se clica no `⋯`.
    fn acoes_do_item(&self, app: &Rc<App>, id: u64, tem_texto: bool) -> GtkBox {
        let caixa = GtkBox::new(Orientation::Horizontal, 4);
        caixa.set_visible(false);

        let acao = |nome: &str, dica: &str| {
            let b = Button::new();
            b.add_css_class("sc-acao");
            b.set_child(Some(&icone(nome)));
            b.set_tooltip_text(Some(dica));
            b.set_focus_on_click(false);
            b
        };

        if tem_texto {
            let texto = acao("supercola-paste-text-symbolic", app.tr("paste_text"));
            let a = app.clone();
            texto.connect_clicked(move |_| a.colar_item(id, true));
            caixa.append(&texto);
        }

        let excluir = acao("supercola-delete-symbolic", app.tr("delete"));
        let a = app.clone();
        excluir.connect_clicked(move |_| {
            a.history.borrow_mut().remove(id);
            a.texturas.borrow_mut().remove(&id);
            a.salvar_fixados();
            a.ao_mudar_historico();
        });
        caixa.append(&excluir);
        caixa
    }

    fn linha(&self, app: &Rc<App>, clip: &Clip) -> ListBoxRow {
        let caixa = GtkBox::new(Orientation::Horizontal, 12);
        caixa.add_css_class("sc-item");
        caixa.set_hexpand(true);

        match (clip.kind(), textura(app, clip)) {
            (Kind::Image, Some(t)) => {
                let p = Picture::for_paintable(&t);
                p.set_content_fit(gtk4::ContentFit::Contain);
                p.set_can_shrink(true);
                let largura = t.width() * ALTURA_MINIATURA / t.height().max(1);
                p.set_size_request(largura.min(LARGURA_MINIATURA), ALTURA_MINIATURA);
                p.set_halign(Align::Start);
                p.set_valign(Align::Center);
                p.set_margin_top(6);
                p.set_margin_bottom(6);
                p.set_alternative_text(Some(app.tr("image")));
                caixa.append(&p);
                // Empurra as ações para a direita, com a miniatura colada à esquerda.
                let resto = GtkBox::new(Orientation::Horizontal, 0);
                resto.set_hexpand(true);
                caixa.append(&resto);
            }
            _ => {
                let l = Label::new(Some(&resumo(clip.text().unwrap_or(""))));
                l.set_xalign(0.0);
                l.set_yalign(0.0);
                l.set_hexpand(true);
                l.set_valign(Align::Start);
                l.set_margin_top(10);
                l.set_wrap(true);
                l.set_wrap_mode(pango::WrapMode::WordChar);
                l.set_lines(LINHAS);
                l.set_ellipsize(pango::EllipsizeMode::End);
                l.set_max_width_chars(1);
                caixa.append(&l);
            }
        }

        let acoes = GtkBox::new(Orientation::Vertical, 0);
        acoes.set_margin_top(6);
        acoes.set_margin_bottom(6);

        let id = clip.id;
        let extras = self.acoes_do_item(app, id, clip.text().is_some());
        self.acoes_abertas.borrow_mut().push(extras.clone());

        let mais = Button::new();
        mais.add_css_class("sc-icon-btn");
        mais.set_child(Some(&icone("supercola-more-symbolic")));
        mais.set_tooltip_text(Some(app.tr("see_more")));
        mais.set_focus_on_click(false);
        let (abertas, e) = (self.acoes_abertas.clone(), extras.clone());
        mais.connect_clicked(move |_| {
            let abrir = !e.is_visible();
            for outra in abertas.borrow().iter() {
                outra.set_visible(false);
            }
            e.set_visible(abrir);
        });
        acoes.append(&mais);

        let pino = Button::new();
        pino.add_css_class("sc-icon-btn");
        pino.set_focus_on_click(false);
        pino.set_vexpand(true);
        pino.set_valign(Align::End);
        let nome = if clip.pinned {
            "supercola-pin-filled-symbolic"
        } else {
            "supercola-pin-symbolic"
        };
        pino.set_child(Some(&icone(nome)));
        pino.set_tooltip_text(Some(app.tr(if clip.pinned { "unpin" } else { "pin" })));
        if clip.pinned {
            pino.add_css_class("sc-pinned");
        }
        let a = app.clone();
        pino.connect_clicked(move |_| {
            a.history.borrow_mut().toggle_pin(id);
            a.salvar_fixados();
            a.ao_mudar_historico();
        });
        acoes.append(&pino);
        caixa.append(&acoes);

        let externa = GtkBox::new(Orientation::Horizontal, 4);
        externa.append(&caixa);
        externa.append(&extras);

        let linha = ListBoxRow::new();
        linha.set_widget_name(&clip.id.to_string());
        linha.set_child(Some(&externa));
        linha
    }

    pub fn refresh(&self, app: &Rc<App>, busca: &str) {
        let selecionado = self.lista.selected_row().and_then(|l| id_da_linha(&l));
        let tinha_foco = self.lista.focus_child().is_some();
        self.acoes_abertas.borrow_mut().clear();
        self.lista.remove_all();
        let historico = app.history.borrow();
        let itens = historico.search(busca);
        let ligado = app.cfg.borrow().history_enabled;
        for clip in &itens {
            let linha = self.linha(app, clip);
            self.lista.append(&linha);
            if Some(clip.id) == selecionado {
                self.lista.select_row(Some(&linha));
            }
        }
        self.limpar.set_visible(!historico.items().is_empty());
        let pagina = if !itens.is_empty() {
            "lista"
        } else if !ligado {
            "desligado"
        } else if busca.trim().is_empty() {
            "vazio"
        } else {
            "sem-resultado"
        };
        self.conteudo.set_visible_child_name(pagina);
        // Refazer as linhas tira o foco delas, e o teclado precisa continuar na lista.
        if tinha_foco {
            if let Some(linha) = self.lista.selected_row() {
                linha.grab_focus();
            }
        }
    }

    pub fn focar(&self) -> bool {
        let Some(linha) = self
            .lista
            .selected_row()
            .or_else(|| self.lista.row_at_index(0))
        else {
            return false;
        };
        self.lista.select_row(Some(&linha));
        linha.grab_focus()
    }

    /// Ao abrir, o teclado começa no item mais recente, não no da última vez.
    pub fn ir_para_o_topo(&self) {
        self.lista.unselect_all();
    }

    pub fn ativar_selecionado(&self, app: &Rc<App>) {
        // A busca refaz as linhas e pode deixar a lista sem seleção.
        let linha = self
            .lista
            .selected_row()
            .or_else(|| self.lista.row_at_index(0));
        if let Some(id) = linha.and_then(|l| id_da_linha(&l)) {
            app.colar_item(id, false);
        }
    }

    pub fn ativar_primeiro(&self, app: &Rc<App>) {
        if let Some(id) = self.lista.row_at_index(0).and_then(|l| id_da_linha(&l)) {
            app.colar_item(id, false);
        }
    }

    pub fn excluir_selecionado(&self, app: &Rc<App>) {
        let foco = self
            .lista
            .root()
            .and_then(|r| gtk4::prelude::RootExt::focus(&r))
            .and_then(|w| w.downcast::<ListBoxRow>().ok());
        let Some(linha) = foco.or_else(|| self.lista.selected_row()) else {
            return;
        };
        let Some(id) = id_da_linha(&linha) else {
            return;
        };
        let indice = linha.index();
        app.history.borrow_mut().remove(id);
        app.texturas.borrow_mut().remove(&id);
        app.salvar_fixados();
        app.ao_mudar_historico();
        // Mantém o teclado na lista, no item que tomou o lugar do excluído.
        let proxima = self
            .lista
            .row_at_index(indice)
            .or_else(|| self.lista.row_at_index(indice - 1));
        if let Some(l) = proxima {
            self.lista.select_row(Some(&l));
            l.grab_focus();
        }
    }
}
