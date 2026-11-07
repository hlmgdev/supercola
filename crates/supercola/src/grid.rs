//! Abas em grade: emoji, kaomoji e símbolos.
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    glib, Align, Box as GtkBox, Button, GridView, Label, ListItem, Orientation, PolicyType,
    Popover, ScrolledWindow, SignalListItemFactory, SingleSelection, StringList, StringObject,
    ToggleButton,
};
use supercola_core::emoji::{self, Emoji};
use supercola_core::ipc::Tab;
use supercola_core::lists::{self, Grupo, KAOMOJI, SIMBOLOS};

use crate::app::App;
use crate::panel::{com_indicador, icone_aba};

const RECENTES: i32 = -1;
const COLUNAS_EMOJI: u32 = 6;
const COLUNAS_KAOMOJI: u32 = 3;
const MARGEM_TITULO_COM_BUSCA: i32 = 14;
const MARGEM_TITULO_SEM_BUSCA: i32 = 34;

/// Categorias do Windows, formadas pelos grupos do emojibase. O grupo 2 são componentes.
const GRUPOS_EMOJI: &[(&[u8], &str, &str)] = &[
    (&[0, 3], "supercola-emoji-symbolic", "emoji_rostos"),
    (&[1], "supercola-people-symbolic", "emoji_pessoas"),
    (&[6, 7], "supercola-activity-symbolic", "emoji_objetos"),
    (&[4], "supercola-food-symbolic", "emoji_comida"),
    (&[5], "supercola-travel-symbolic", "emoji_lugares"),
    (&[8, 9], "supercola-heart-symbolic", "emoji_simbolos"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tipo {
    Emoji,
    Kaomoji,
    Symbols,
}

impl Tipo {
    fn aba(self) -> Tab {
        match self {
            Self::Emoji => Tab::Emoji,
            Self::Kaomoji => Tab::Kaomoji,
            Self::Symbols => Tab::Symbols,
        }
    }
    fn grupos(self) -> &'static [Grupo] {
        match self {
            Self::Kaomoji => KAOMOJI,
            _ => SIMBOLOS,
        }
    }
}

struct Estado {
    tipo: Tipo,
    titulo: Label,
    modelo: StringList,
    grupo: Cell<i32>,
    tom: Cell<usize>,
    busca: RefCell<String>,
    emojis: OnceCell<Vec<Emoji>>,
    botoes: RefCell<Vec<(i32, ToggleButton)>>,
    atualizando: Cell<bool>,
}

pub struct Grid {
    pub root: GtkBox,
    /// Categorias, que o painel mostra no lugar das abas.
    pub barra: ScrolledWindow,
    cabecalho: GtkBox,
    vista: GridView,
    selecao: SingleSelection,
    estado: Rc<Estado>,
}

impl Estado {
    fn lista_emojis(&self, app: &App) -> &Vec<Emoji> {
        self.emojis.get_or_init(|| emoji::all(app.lang.get()))
    }

    fn itens(&self, app: &App) -> Vec<String> {
        let busca = self.busca.borrow();
        let tom = self.tom.get();
        if self.tipo == Tipo::Emoji {
            let todos = self.lista_emojis(app);
            if !busca.trim().is_empty() {
                return emoji::search(todos, &busca)
                    .iter()
                    .map(|e| e.with_tone(tom).to_string())
                    .collect();
            }
            if self.grupo.get() == RECENTES {
                return app.recent.borrow().list(Tab::Emoji.name()).to_vec();
            }
            let (grupos, _, _) = GRUPOS_EMOJI[self.grupo.get() as usize];
            return todos
                .iter()
                .filter(|e| grupos.contains(&e.group))
                .map(|e| e.with_tone(tom).to_string())
                .collect();
        }
        if !busca.trim().is_empty() {
            return lists::buscar(self.tipo.grupos(), &busca)
                .iter()
                .map(|s| s.to_string())
                .collect();
        }
        if self.grupo.get() == RECENTES {
            return app.recent.borrow().list(self.tipo.aba().name()).to_vec();
        }
        self.tipo.grupos()[self.grupo.get() as usize]
            .itens
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn nome_do_grupo(&self, app: &App) -> &'static str {
        let grupo = self.grupo.get();
        if !self.busca.borrow().trim().is_empty() {
            app.tr("search_results")
        } else if grupo == RECENTES {
            app.tr("tab_home")
        } else if self.tipo == Tipo::Emoji {
            app.tr(GRUPOS_EMOJI[grupo as usize].2)
        } else {
            app.tr(self.tipo.grupos()[grupo as usize].nome)
        }
    }

    fn atualizar(&self, app: &App) {
        self.titulo.set_text(self.nome_do_grupo(app));
        let itens = self.itens(app);
        let refs: Vec<&str> = itens.iter().map(String::as_str).collect();
        self.modelo.splice(0, self.modelo.n_items(), &refs);
    }

    fn marcar(&self) {
        self.atualizando.set(true);
        for (g, b) in self.botoes.borrow().iter() {
            b.set_active(*g == self.grupo.get());
        }
        self.atualizando.set(false);
    }
}

pub fn botao_grupo(filho: &impl IsA<gtk4::Widget>, dica: &str) -> ToggleButton {
    let b = ToggleButton::new();
    b.add_css_class("sc-group");
    b.set_child(Some(&com_indicador(filho)));
    b.set_tooltip_text(Some(dica));
    b.set_focus_on_click(false);
    b
}

impl Grid {
    pub fn new(app: &Rc<App>, tipo: Tipo) -> Self {
        let root = GtkBox::new(Orientation::Vertical, 0);
        let modelo = StringList::new(&[]);
        let selecao = SingleSelection::new(Some(modelo.clone()));
        selecao.set_autoselect(false);
        selecao.set_can_unselect(true);

        let classe = match tipo {
            Tipo::Emoji => "sc-cell-emoji",
            Tipo::Kaomoji => "sc-cell-kaomoji",
            Tipo::Symbols => "sc-cell-symbol",
        };
        let fabrica = SignalListItemFactory::new();
        fabrica.connect_setup(move |_, item| {
            let l = Label::new(None);
            l.add_css_class("sc-cell");
            l.add_css_class(classe);
            if tipo == Tipo::Kaomoji {
                l.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                l.set_max_width_chars(12);
            }
            item.downcast_ref::<ListItem>()
                .expect("ListItem")
                .set_child(Some(&l));
        });
        fabrica.connect_bind(|_, item| {
            let item = item.downcast_ref::<ListItem>().expect("ListItem");
            let texto = item
                .item()
                .and_downcast::<StringObject>()
                .map(|s| s.string());
            if let (Some(l), Some(t)) = (item.child().and_downcast::<Label>(), texto) {
                l.set_text(&t);
                l.set_tooltip_text(Some(&t));
            }
        });

        let vista = GridView::new(Some(selecao.clone()), Some(fabrica));
        vista.add_css_class("sc-grid");
        vista.set_single_click_activate(true);
        let colunas = if tipo == Tipo::Kaomoji {
            COLUNAS_KAOMOJI
        } else {
            COLUNAS_EMOJI
        };
        vista.set_min_columns(colunas);
        vista.set_max_columns(colunas);
        vista.set_margin_start(16);
        vista.set_margin_end(16);

        let a = app.clone();
        let m = modelo.clone();
        vista.connect_activate(move |_, pos| {
            if let Some(texto) = m.string(pos) {
                a.escolher(tipo.aba(), &texto);
            }
        });

        let rolagem = ScrolledWindow::new();
        rolagem.set_hscrollbar_policy(PolicyType::Never);
        rolagem.set_vexpand(true);
        rolagem.set_child(Some(&vista));

        let cabecalho = GtkBox::new(Orientation::Horizontal, 0);
        cabecalho.set_margin_top(MARGEM_TITULO_COM_BUSCA);
        cabecalho.set_margin_start(16);
        cabecalho.set_margin_end(16);
        cabecalho.set_margin_bottom(8);
        let titulo = Label::new(None);
        titulo.add_css_class("sc-title");
        titulo.set_hexpand(true);
        titulo.set_xalign(0.0);
        titulo.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        cabecalho.append(&titulo);
        root.append(&cabecalho);
        root.append(&rolagem);

        let estado = Rc::new(Estado {
            tipo,
            titulo,
            modelo,
            grupo: Cell::new(0),
            tom: Cell::new(0),
            busca: RefCell::default(),
            emojis: OnceCell::new(),
            botoes: RefCell::default(),
            atualizando: Cell::new(false),
        });

        let categorias = GtkBox::new(Orientation::Horizontal, 0);

        let mut grupos: Vec<(i32, ToggleButton)> = Vec::new();
        grupos.push((
            RECENTES,
            botao_grupo(&icone_aba("supercola-recent-symbolic"), app.tr("recent")),
        ));
        if tipo == Tipo::Emoji {
            for (i, (_, nome, chave)) in GRUPOS_EMOJI.iter().enumerate() {
                grupos.push((i as i32, botao_grupo(&icone_aba(nome), app.tr(chave))));
            }
        } else {
            for (i, g) in tipo.grupos().iter().enumerate() {
                let nome = app.tr(g.nome);
                let b = botao_grupo(&Label::new(Some(nome)), nome);
                b.add_css_class("sc-group-texto");
                grupos.push((i as i32, b));
            }
        }
        for (g, b) in &grupos {
            categorias.append(b);
            let (g, a, e) = (*g, app.clone(), estado.clone());
            b.connect_toggled(move |b| {
                if e.atualizando.get() {
                    return;
                }
                if b.is_active() {
                    e.grupo.set(g);
                    e.marcar();
                    e.atualizar(&a);
                } else if e.grupo.get() == g {
                    // Um grupo fica sempre marcado.
                    e.marcar();
                }
            });
        }
        *estado.botoes.borrow_mut() = grupos;

        if tipo == Tipo::Emoji {
            cabecalho.append(&botao_tom(app, &estado));
        }

        // Os nomes das categorias não cabem todos: a linha rola, sem barra visível.
        let barra = ScrolledWindow::new();
        barra.set_policy(PolicyType::External, PolicyType::Never);
        barra.set_hexpand(true);
        barra.set_child(Some(&categorias));

        Self {
            root,
            barra,
            cabecalho,
            vista,
            selecao,
            estado,
        }
    }

    /// Sem o campo de busca, o título sobe para o lugar que ele ocuparia.
    pub fn ajustar_titulo(&self, com_busca: bool) {
        self.cabecalho.set_margin_top(if com_busca {
            MARGEM_TITULO_COM_BUSCA
        } else {
            MARGEM_TITULO_SEM_BUSCA
        });
    }

    pub fn filtrar(&self, app: &Rc<App>, texto: &str) {
        *self.estado.busca.borrow_mut() = texto.to_string();
        if texto.is_empty() {
            let tem_recentes = !app
                .recent
                .borrow()
                .list(self.estado.tipo.aba().name())
                .is_empty();
            if self.estado.grupo.get() == RECENTES && !tem_recentes {
                self.estado.grupo.set(0);
            }
            self.estado.marcar();
        }
        self.estado.atualizar(app);
        // Mudar a lista invalida a seleção. Como no Windows, o primeiro resultado fica
        // marcado, e o Enter o escolhe.
        if self.estado.modelo.n_items() > 0 {
            self.selecao.set_selected(0);
        }
    }

    pub fn ativar_primeiro(&self, app: &Rc<App>) {
        if let Some(texto) = self.estado.modelo.string(0) {
            app.escolher(self.estado.tipo.aba(), &texto);
        }
    }

    pub fn ativar_selecionado(&self, app: &Rc<App>) {
        let selecionado = self.estado.modelo.string(self.selecao.selected());
        if let Some(texto) = selecionado.or_else(|| self.estado.modelo.string(0)) {
            app.escolher(self.estado.tipo.aba(), &texto);
        }
    }

    pub fn focar(&self) -> bool {
        if self.estado.modelo.n_items() == 0 {
            return false;
        }
        self.selecao.set_selected(0);
        self.vista.grab_focus()
    }
}

fn botao_tom(app: &Rc<App>, estado: &Rc<Estado>) -> Button {
    let b = Button::new();
    b.add_css_class("sc-botao-seta");
    b.set_tooltip_text(Some(app.tr("skin_tone")));
    b.set_focus_on_click(false);
    let ponto = GtkBox::new(Orientation::Horizontal, 0);
    ponto.set_halign(Align::Center);
    ponto.set_valign(Align::Center);
    ponto.add_css_class("sc-tone-dot");
    ponto.add_css_class("sc-tone-0");
    b.set_child(Some(&ponto));

    let (a, e) = (app.clone(), estado.clone());
    let p0 = ponto.clone();
    b.connect_clicked(move |b| {
        let pop = Popover::new();
        pop.add_css_class("sc-menu");
        pop.set_has_arrow(false);
        pop.set_parent(b);
        let fila = GtkBox::new(Orientation::Horizontal, 4);
        for tom in 0..=5usize {
            let opcao = Button::new();
            opcao.add_css_class("sc-tone");
            opcao.add_css_class(&format!("sc-tone-{tom}"));
            let (a, e, p, ponto) = (a.clone(), e.clone(), pop.clone(), p0.clone());
            opcao.connect_clicked(move |_| {
                e.tom.set(tom);
                for t in 0..=5 {
                    ponto.remove_css_class(&format!("sc-tone-{t}"));
                }
                ponto.add_css_class(&format!("sc-tone-{tom}"));
                e.atualizar(&a);
                p.popdown();
            });
            fila.append(&opcao);
        }
        pop.set_child(Some(&fila));
        let a2 = a.clone();
        pop.connect_closed(move |p| {
            a2.popover_aberto.set(false);
            let p = p.clone();
            glib::idle_add_local_once(move || p.unparent());
        });
        a.popover_aberto.set(true);
        pop.popup();
    });
    b
}
