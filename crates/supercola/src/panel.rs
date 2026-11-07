//! Painel: superfície transparente de tela cheia, com o cartão do Win+V dentro.
//!
//! A superfície inteira existe para capturar o clique fora, que uma layer surface
//! pequena com foco exclusivo não recebe.
//!
//! No COSMIC, com o painel no canto, o clique fora também chega ao app de trás, como no
//! Windows: o foco exclusivo vale só ao abrir, e a entrada fica limitada ao cartão. Aí o
//! COSMIC move o foco ao receber o clique, e o painel fecha ao perdê-lo. Em wlroots o foco
//! segue o mouse, então lá o painel continua lendo o clique com a superfície inteira.
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use gtk4::prelude::*;
use gtk4::{
    cairo, gdk, glib, Align, ApplicationWindow, Box as GtkBox, Button, CenterBox, Image, Label,
    Orientation, SearchEntry, Stack, StackTransitionType, ToggleButton, Widget,
};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use supercola_core::config::Position;
use supercola_core::ipc::Tab;
use supercola_wayland::probe::Ambiente;

use crate::app::App;
use crate::gif::GifTab;
use crate::grid::{Grid, Tipo};
use crate::history::HistoryTab;
use crate::home::HomeTab;

/// A extensão do GNOME reconhece o painel por este título.
const TITULO: &str = "Super Cola";
const LARGURA: i32 = 360;
const ALTURA: i32 = 400;
const MARGEM: i32 = 12;
/// Tempo com foco exclusivo ao abrir. Sem ele, o COSMIC pode não entregar o teclado se
/// um menu de outro app estiver aberto.
const FOCO_EXCLUSIVO: Duration = Duration::from_millis(250);
/// Espera antes de tratar a perda de foco como clique fora: a troca de modo do teclado
/// pode tirar o foco por um instante.
const ASSENTAR_FOCO: Duration = Duration::from_millis(100);
const TAMANHO_ICONE: i32 = 16;
/// Os ícones da Fluent têm folga interna: em 20 px o desenho fica com 16 px, como no Windows.
const TAMANHO_ICONE_ABA: i32 = 20;

pub struct Ui {
    pub window: ApplicationWindow,
    pub search: SearchEntry,
    pub tab: Cell<Tab>,
    pub history: HistoryTab,
    card: GtkBox,
    topo: CenterBox,
    fechar: Button,
    nav: Stack,
    stack: Stack,
    tabs: Vec<(Tab, ToggleButton)>,
    home: HomeTab,
    emoji: Grid,
    kaomoji: Grid,
    symbols: Grid,
    gif: GifTab,
    /// Evita tratar como clique do usuário a troca de aba feita pelo código.
    trocando: Cell<bool>,
    /// O último filtro aplicado. Repetir o filtro refaria as linhas e perderia o foco.
    filtro: RefCell<String>,
    /// Canto do cartão no início do arraste.
    origem: Cell<(f64, f64)>,
    /// O clique fora já passa para o app de trás, e a perda de foco fecha o painel.
    clique_livre: Cell<bool>,
    /// No GNOME, que não tem layer-shell, o painel é uma janela do tamanho do cartão.
    janela_comum: bool,
}

pub fn icone(nome: &str) -> Image {
    let i = Image::from_icon_name(nome);
    i.set_pixel_size(TAMANHO_ICONE);
    i
}

pub fn icone_aba(nome: &str) -> Image {
    let i = Image::from_icon_name(nome);
    i.set_pixel_size(TAMANHO_ICONE_ABA);
    i
}

/// Conteúdo de aba ou categoria, com o traço que marca a opção ativa.
pub fn com_indicador(filho: &impl IsA<Widget>) -> GtkBox {
    let caixa = GtkBox::new(Orientation::Vertical, 0);
    filho.set_vexpand(true);
    filho.set_valign(Align::Center);
    caixa.append(filho);
    let traco = GtkBox::new(Orientation::Horizontal, 0);
    traco.add_css_class("sc-indicador");
    traco.set_halign(Align::Center);
    caixa.append(&traco);
    caixa
}

fn botao_aba(app: &App, aba: Tab) -> ToggleButton {
    let b = ToggleButton::new();
    b.add_css_class("sc-tab");
    b.set_focus_on_click(false);
    let (figura, chave): (Widget, _) = match aba {
        Tab::Home => (icone_aba("supercola-home-symbolic").upcast(), "tab_home"),
        Tab::Emoji => (icone_aba("supercola-emoji-symbolic").upcast(), "tab_emoji"),
        Tab::Gif => (icone_aba("supercola-gif-symbolic").upcast(), "tab_gif"),
        // O Windows também desenha esta aba como texto.
        Tab::Kaomoji => (Label::new(Some(";-)")).upcast(), "tab_kaomoji"),
        Tab::Symbols => (
            icone_aba("supercola-symbols-symbolic").upcast(),
            "tab_symbols",
        ),
        Tab::History => (
            icone_aba("supercola-clipboard-symbolic").upcast(),
            "tab_history",
        ),
    };
    b.set_child(Some(&com_indicador(&figura)));
    b.set_tooltip_text(Some(app.tr(chave)));
    b
}

/// Linha que toma o lugar das abas: seta de voltar e categorias.
fn subnav(app: &App, categorias: &impl IsA<Widget>) -> (GtkBox, Button) {
    let linha = GtkBox::new(Orientation::Horizontal, 0);
    linha.set_margin_start(15);
    let voltar = Button::new();
    voltar.add_css_class("sc-tab");
    voltar.set_child(Some(&icone_aba("supercola-back-symbolic")));
    voltar.set_tooltip_text(Some(app.tr("back")));
    voltar.set_focus_on_click(false);
    linha.append(&voltar);
    linha.append(categorias);
    (linha, voltar)
}

fn busca_sempre_visivel(aba: Tab) -> bool {
    matches!(aba, Tab::Home | Tab::Emoji | Tab::Gif)
}

impl Ui {
    pub fn new(app: &Rc<App>) -> Rc<Self> {
        let window = ApplicationWindow::new(&app.gtk_app);
        let janela_comum = app.ambiente == Ambiente::Gnome;
        if janela_comum {
            window.set_title(Some(TITULO));
            window.set_decorated(false);
            window.set_resizable(false);
        } else {
            window.init_layer_shell();
            window.set_layer(Layer::Overlay);
            window.set_keyboard_mode(KeyboardMode::Exclusive);
            window.set_namespace(Some("supercola"));
            // Zona 0: respeita painel e dock, e o cartão não fica atrás deles.
            window.set_exclusive_zone(0);
            for borda in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
                window.set_anchor(borda, true);
            }
        }
        window.add_css_class("supercola-surface");

        let card = GtkBox::new(Orientation::Vertical, 0);
        card.add_css_class("sc-card-root");
        card.set_size_request(LARGURA, ALTURA);
        card.set_overflow(gtk4::Overflow::Hidden);
        card.set_halign(Align::End);
        card.set_valign(Align::End);
        card.set_margin_end(MARGEM);
        card.set_margin_bottom(MARGEM);
        card.set_margin_top(MARGEM);
        card.set_margin_start(MARGEM);
        window.set_child(Some(&card));

        let topo = CenterBox::new();
        topo.add_css_class("sc-topo");
        let alca = GtkBox::new(Orientation::Horizontal, 0);
        alca.add_css_class("sc-alca");
        alca.set_valign(Align::Center);
        topo.set_center_widget(Some(&alca));
        let fechar = Button::new();
        fechar.add_css_class("sc-icon-btn");
        fechar.set_child(Some(&icone("supercola-dismiss-symbolic")));
        fechar.set_tooltip_text(Some(app.tr("close")));
        fechar.set_focus_on_click(false);
        fechar.set_valign(Align::Center);
        fechar.set_margin_end(8);
        topo.set_end_widget(Some(&fechar));
        card.append(&topo);

        let barra = GtkBox::new(Orientation::Horizontal, 0);
        barra.set_margin_start(15);
        let mut tabs = Vec::new();
        for aba in Tab::ALL {
            let b = botao_aba(app, aba);
            if let Some((_, primeiro)) = tabs.first() {
                b.set_group(Some::<&ToggleButton>(primeiro));
            }
            barra.append(&b);
            tabs.push((aba, b));
        }
        let nav = Stack::new();
        // O conteúdo de cada botão se expande, e isso não pode subir até aqui.
        nav.set_vexpand(false);
        nav.set_margin_top(8);
        nav.set_transition_type(StackTransitionType::None);
        nav.add_named(&barra, Some("principal"));
        card.append(&nav);

        let search = SearchEntry::new();
        search.add_css_class("sc-search");
        search.set_placeholder_text(Some(app.tr("search")));
        search.set_search_delay(60);
        search.set_margin_top(20);
        search.set_margin_start(16);
        search.set_margin_end(16);
        search.set_margin_bottom(8);
        search.set_key_capture_widget(Some(&window));
        card.append(&search);

        let stack = Stack::new();
        stack.set_vexpand(true);
        stack.set_transition_type(StackTransitionType::None);
        card.append(&stack);

        let home = HomeTab::new(app);
        let history = HistoryTab::new(app);
        let emoji = Grid::new(app, Tipo::Emoji);
        let kaomoji = Grid::new(app, Tipo::Kaomoji);
        let symbols = Grid::new(app, Tipo::Symbols);
        let gif = GifTab::new(app);
        stack.add_named(&home.root, Some(Tab::Home.name()));
        stack.add_named(&history.root, Some(Tab::History.name()));
        stack.add_named(&emoji.root, Some(Tab::Emoji.name()));
        stack.add_named(&kaomoji.root, Some(Tab::Kaomoji.name()));
        stack.add_named(&symbols.root, Some(Tab::Symbols.name()));
        stack.add_named(&gif.root, Some(Tab::Gif.name()));

        let mut voltar = Vec::new();
        let mut linha_de = |aba: Tab, categorias: &Widget| {
            let (linha, botao) = subnav(app, categorias);
            nav.add_named(&linha, Some(aba.name()));
            voltar.push(botao);
        };
        linha_de(Tab::Emoji, emoji.barra.upcast_ref());
        linha_de(Tab::Kaomoji, kaomoji.barra.upcast_ref());
        linha_de(Tab::Symbols, symbols.barra.upcast_ref());
        linha_de(Tab::Gif, gif.barra.upcast_ref());

        let ui = Rc::new(Ui {
            window,
            search,
            tab: Cell::new(Tab::History),
            history,
            card,
            topo,
            fechar,
            nav,
            stack,
            tabs,
            home,
            emoji,
            kaomoji,
            symbols,
            gif,
            trocando: Cell::new(false),
            filtro: RefCell::default(),
            origem: Cell::new((0.0, 0.0)),
            clique_livre: Cell::new(false),
            janela_comum,
        });
        ui.ligar(app, &voltar);
        ui.ligar_arraste(app);
        ui.ligar_perda_de_foco(app);
        ui
    }

    fn ligar(self: &Rc<Self>, app: &Rc<App>, voltar: &[Button]) {
        for (aba, botao) in &self.tabs {
            let (aba, ui, a) = (*aba, Rc::downgrade(self), app.clone());
            botao.connect_toggled(move |b| {
                let Some(ui) = ui.upgrade() else { return };
                if b.is_active() && !ui.trocando.get() {
                    ui.selecionar(&a, aba);
                }
            });
        }

        let a = app.clone();
        self.fechar.connect_clicked(move |_| a.esconder());

        for botao in voltar {
            let (ui, a) = (Rc::downgrade(self), app.clone());
            botao.connect_clicked(move |_| {
                if let Some(ui) = ui.upgrade() {
                    ui.selecionar(&a, Tab::Home);
                }
            });
        }

        // Sem o atraso da busca: o campo aparece junto com a primeira letra.
        let (ui, a) = (Rc::downgrade(self), app.clone());
        self.search.connect_changed(move |_| {
            if let Some(ui) = ui.upgrade() {
                ui.ajustar_busca(&a);
            }
        });

        let (ui, a) = (Rc::downgrade(self), app.clone());
        self.search.connect_search_changed(move |s| {
            let Some(ui) = ui.upgrade() else { return };
            let texto = s.text();
            if ui.tab.get() == Tab::Home && !texto.is_empty() {
                ui.buscar_emoji(&a, &texto);
            } else if *ui.filtro.borrow() != texto.as_str() {
                ui.filtrar(&a, &texto);
            }
        });

        let (ui, a) = (Rc::downgrade(self), app.clone());
        self.search.connect_activate(move |_| {
            if let Some(ui) = ui.upgrade() {
                ui.ativar_primeiro(&a);
            }
        });

        // Fase de captura: Esc e as setas precisam funcionar com o foco na busca.
        let teclas = gtk4::EventControllerKey::new();
        teclas.set_propagation_phase(gtk4::PropagationPhase::Capture);
        let (ui, a) = (Rc::downgrade(self), app.clone());
        teclas.connect_key_pressed(move |_, tecla, _, _| {
            let Some(ui) = ui.upgrade() else {
                return glib::Propagation::Proceed;
            };
            let na_busca = ui.search.has_focus() || ui.search.focus_child().is_some();
            match tecla {
                gdk::Key::Escape => {
                    if ui.search.text().is_empty() {
                        a.esconder();
                    } else {
                        ui.search.set_text("");
                        ui.filtrar(&a, "");
                        ui.focar_conteudo();
                    }
                    glib::Propagation::Stop
                }
                gdk::Key::Delete | gdk::Key::KP_Delete
                    if ui.tab.get() == Tab::History && !na_busca =>
                {
                    ui.history.excluir_selecionado(&a);
                    glib::Propagation::Stop
                }
                gdk::Key::Down if na_busca => {
                    ui.focar_conteudo();
                    glib::Propagation::Stop
                }
                // A busca captura o teclado da janela e ativaria o primeiro resultado.
                gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::ISO_Enter if !na_busca => {
                    ui.ativar_selecionado(&a);
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        self.window.add_controller(teclas);

        let clique = gtk4::GestureClick::new();
        let (ui, a) = (Rc::downgrade(self), app.clone());
        clique.connect_released(move |_, _, x, y| {
            let Some(ui) = ui.upgrade() else { return };
            let dentro = ui
                .window
                .pick(x, y, gtk4::PickFlags::DEFAULT)
                .is_some_and(|w| {
                    w == *ui.card.upcast_ref::<gtk4::Widget>() || w.is_ancestor(&ui.card)
                });
            if !dentro {
                a.esconder();
            }
        });
        self.window.add_controller(clique);

        let movimento = gtk4::EventControllerMotion::new();
        let (ui, a) = (Rc::downgrade(self), app.clone());
        movimento.connect_enter(move |_, x, y| {
            if let Some(ui) = ui.upgrade() {
                if a.cfg.borrow().position == Position::NearPointer && !ui.janela_comum {
                    ui.perto_do_ponteiro(x, y);
                    ui.acompanhar_desfoque(&a);
                }
            }
        });
        self.window.add_controller(movimento);
    }

    /// O gesto fica na janela, que não se move: dentro do cartão, as coordenadas
    /// mudariam junto com ele.
    fn ligar_arraste(self: &Rc<Self>, app: &Rc<App>) {
        let arraste = gtk4::GestureDrag::new();

        let ui = Rc::downgrade(self);
        arraste.connect_drag_begin(move |g, x, y| {
            let Some(ui) = ui.upgrade() else { return };
            let na_alca = ui
                .window
                .pick(x, y, gtk4::PickFlags::DEFAULT)
                .is_some_and(|w| {
                    let no_topo = w == *ui.topo.upcast_ref::<Widget>() || w.is_ancestor(&ui.topo);
                    let no_fechar =
                        w == *ui.fechar.upcast_ref::<Widget>() || w.is_ancestor(&ui.fechar);
                    no_topo && !no_fechar
                });
            let canto = ui.card.compute_bounds(&ui.window);
            let (true, Some(canto)) = (na_alca, canto) else {
                g.set_state(gtk4::EventSequenceState::Denied);
                return;
            };
            if ui.janela_comum {
                ui.mover_janela(g, x, y);
                g.set_state(gtk4::EventSequenceState::Claimed);
                return;
            }
            ui.origem.set((f64::from(canto.x()), f64::from(canto.y())));
            // Fica com o toque: soltar não pode contar como clique fora.
            g.set_state(gtk4::EventSequenceState::Claimed);
        });

        let (ui, a) = (Rc::downgrade(self), app.clone());
        arraste.connect_drag_update(move |_, dx, dy| {
            if let Some(ui) = ui.upgrade().filter(|u| !u.janela_comum) {
                let (x, y) = ui.origem.get();
                ui.posicionar((x + dx) as i32, (y + dy) as i32);
                ui.acompanhar_desfoque(&a);
            }
        });

        let ui = Rc::downgrade(self);
        arraste.connect_drag_end(move |_, _, _| {
            if let Some(ui) = ui.upgrade() {
                if ui.clique_livre.get() {
                    ui.limitar_entrada_ao_cartao();
                }
            }
        });
        self.window.add_controller(arraste);
    }

    /// Em janela comum, quem move é o compositor.
    fn mover_janela(&self, gesto: &gtk4::GestureDrag, x: f64, y: f64) {
        let superficie = self.window.surface();
        let topo = superficie.and_downcast_ref::<gdk::Toplevel>();
        if let (Some(topo), Some(dispositivo)) = (topo, gesto.current_event_device()) {
            topo.begin_move(
                &dispositivo,
                gesto.current_button() as i32,
                x,
                y,
                gesto.current_event_time(),
            );
        }
    }

    fn ligar_perda_de_foco(self: &Rc<Self>, app: &Rc<App>) {
        let (ui, a) = (Rc::downgrade(self), app.clone());
        self.window.connect_is_active_notify(move |j| {
            let Some(u) = ui.upgrade() else { return };
            if j.is_active() || !u.clique_livre.get() {
                return;
            }
            let (ui, a) = (ui.clone(), a.clone());
            glib::timeout_add_local_once(ASSENTAR_FOCO, move || {
                let Some(u) = ui.upgrade() else { return };
                if u.clique_livre.get() && !u.window.is_active() && !a.popover_aberto.get() {
                    a.esconder();
                }
            });
        });
    }

    /// A posição do cartão só vale depois do layout, e a região precisa segui-lo.
    fn acompanhar_desfoque(self: &Rc<Self>, app: &Rc<App>) {
        if app.desfoque.is_none() {
            return;
        }
        let (ui, a) = (Rc::downgrade(self), app.clone());
        self.card.add_tick_callback(move |card, _| {
            let Some(ui) = ui.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if card.width() == 0 {
                return glib::ControlFlow::Continue;
            }
            let bordas = card.compute_bounds(&ui.window);
            if let (Some(d), Some(s), Some(b)) = (&a.desfoque, ui.window.surface(), bordas) {
                d.aplicar(
                    &s,
                    b.x() as i32,
                    b.y() as i32,
                    b.width() as i32,
                    b.height() as i32,
                );
                // O pedido só vale no próximo `commit` da superfície.
                card.queue_draw();
            }
            glib::ControlFlow::Break
        });
    }

    fn entrada_em(&self, regiao: &cairo::Region) {
        if let Some(superficie) = self.window.surface() {
            superficie.set_input_region(regiao);
        }
    }

    fn limitar_entrada_ao_cartao(&self) {
        if let Some(b) = self.card.compute_bounds(&self.window) {
            self.entrada_em(&cairo::Region::create_rectangle(&cairo::RectangleInt::new(
                b.x() as i32,
                b.y() as i32,
                b.width() as i32,
                b.height() as i32,
            )));
        }
    }

    fn liberar_entrada(&self) {
        // Maior que qualquer tela: o compositor recorta na superfície.
        self.entrada_em(&cairo::Region::create_rectangle(&cairo::RectangleInt::new(
            0,
            0,
            i32::from(i16::MAX),
            i32::from(i16::MAX),
        )));
    }

    fn agendar_clique_livre(self: &Rc<Self>) {
        let ui = Rc::downgrade(self);
        glib::timeout_add_local_once(FOCO_EXCLUSIVO, move || {
            let Some(ui) = ui.upgrade() else { return };
            if ui.window.is_visible() {
                ui.window.set_keyboard_mode(KeyboardMode::OnDemand);
                ui.limitar_entrada_ao_cartao();
                ui.clique_livre.set(true);
            }
        });
    }

    fn posicionar(&self, x: i32, y: i32) {
        let (l, a) = (self.window.width(), self.window.height());
        let cx = x.clamp(MARGEM, (l - LARGURA - MARGEM).max(MARGEM));
        let cy = y.clamp(MARGEM, (a - ALTURA - MARGEM).max(MARGEM));
        self.card.set_halign(Align::Start);
        self.card.set_valign(Align::Start);
        self.card.set_margin_start(cx);
        self.card.set_margin_top(cy);
        self.card.set_margin_end(0);
        self.card.set_margin_bottom(0);
    }

    fn perto_do_ponteiro(&self, x: f64, y: f64) {
        self.posicionar(x as i32, y as i32 - ALTURA);
    }

    fn no_canto(&self) {
        self.card.set_halign(Align::End);
        self.card.set_valign(Align::End);
        self.card.set_margin_start(MARGEM);
        self.card.set_margin_top(MARGEM);
        self.card.set_margin_end(MARGEM);
        self.card.set_margin_bottom(MARGEM);
    }

    /// Só a parte visual da troca de aba: marca o botão e mostra as linhas certas.
    fn definir_aba(&self, aba: Tab) {
        self.tab.set(aba);
        self.trocando.set(true);
        if let Some((_, b)) = self.tabs.iter().find(|(t, _)| *t == aba) {
            b.set_active(true);
        }
        self.trocando.set(false);
        self.stack.set_visible_child_name(aba.name());
        self.nav.set_visible_child_name(match aba {
            Tab::Home | Tab::History => "principal",
            outra => outra.name(),
        });
    }

    fn mostrar_aba(&self, app: &Rc<App>, aba: Tab) {
        self.definir_aba(aba);
        if !self.search.text().is_empty() {
            self.search.set_text("");
        }
        self.ajustar_busca(app);
        self.filtrar(app, "");
    }

    /// Abre a aba e deixa o teclado nela, para os botões que levam a outra aba.
    pub fn selecionar(self: &Rc<Self>, app: &Rc<App>, aba: Tab) {
        self.mostrar_aba(app, aba);
        self.focar(app);
    }

    /// A busca do início mostra os emojis, sem apagar o que já foi digitado.
    fn buscar_emoji(&self, app: &Rc<App>, texto: &str) {
        self.definir_aba(Tab::Emoji);
        self.ajustar_busca(app);
        self.filtrar(app, texto);
    }

    /// Fora de Início, Emoji e GIF, a busca só aparece enquanto há texto. Na área de
    /// transferência ela ocupa o lugar do cabeçalho, para a lista não mudar de lugar.
    fn ajustar_busca(&self, app: &App) {
        let aba = self.tab.get();
        let visivel = busca_sempre_visivel(aba) || !self.search.text().is_empty();
        self.search.set_visible(visivel);
        self.history.mostrar_cabecalho(!visivel);
        self.kaomoji.ajustar_titulo(visivel);
        self.symbols.ajustar_titulo(visivel);
        let dica = match aba {
            Tab::Emoji => "search_emoji",
            Tab::Gif => "search_gif",
            _ => "search",
        };
        self.search.set_placeholder_text(Some(app.tr(dica)));
    }

    fn filtrar(&self, app: &Rc<App>, texto: &str) {
        *self.filtro.borrow_mut() = texto.to_string();
        match self.tab.get() {
            Tab::Home => self.home.atualizar(app),
            Tab::History => self.history.refresh(app, texto),
            Tab::Emoji => self.emoji.filtrar(app, texto),
            Tab::Kaomoji => self.kaomoji.filtrar(app, texto),
            Tab::Symbols => self.symbols.filtrar(app, texto),
            Tab::Gif => self.gif.buscar(app, texto),
        }
    }

    fn ativar_primeiro(&self, app: &Rc<App>) {
        match self.tab.get() {
            Tab::History => self.history.ativar_primeiro(app),
            Tab::Emoji => self.emoji.ativar_primeiro(app),
            Tab::Kaomoji => self.kaomoji.ativar_primeiro(app),
            Tab::Symbols => self.symbols.ativar_primeiro(app),
            Tab::Home | Tab::Gif => {}
        }
    }

    fn ativar_selecionado(&self, app: &Rc<App>) {
        match self.tab.get() {
            Tab::Home => self.home.ativar_selecionado(),
            Tab::History => self.history.ativar_selecionado(app),
            Tab::Emoji => self.emoji.ativar_selecionado(app),
            Tab::Kaomoji => self.kaomoji.ativar_selecionado(app),
            Tab::Symbols => self.symbols.ativar_selecionado(app),
            Tab::Gif => self.gif.ativar_selecionado(app),
        }
    }

    fn focar_conteudo(&self) -> bool {
        match self.tab.get() {
            Tab::Home => self.home.focar(),
            Tab::History => self.history.focar(),
            Tab::Emoji => self.emoji.focar(),
            Tab::Kaomoji => self.kaomoji.focar(),
            Tab::Symbols => self.symbols.focar(),
            Tab::Gif => self.gif.focar(),
        }
    }

    fn focar(self: &Rc<Self>, _app: &Rc<App>) {
        // Depois do `present`: a superfície precisa existir para receber o foco.
        let ui = Rc::downgrade(self);
        glib::idle_add_local_once(move || {
            if let Some(ui) = ui.upgrade() {
                if !ui.focar_conteudo() && ui.search.is_visible() {
                    ui.search.grab_focus();
                }
            }
        });
    }

    pub fn abrir(self: &Rc<Self>, app: &Rc<App>, aba: Tab) {
        app.reaplicar_tema();
        self.no_canto();
        self.history.ir_para_o_topo();
        self.mostrar_aba(app, aba);

        if app.cfg.borrow().position == Position::NearPointer {
            // A posição do ponteiro só chega com a superfície aberta.
            self.card.set_opacity(0.0);
            let card = self.card.clone();
            glib::timeout_add_local_once(Duration::from_millis(60), move || card.set_opacity(1.0));
        }
        if let Some(shell) = &app.shell {
            shell.posicionar(app.cfg.borrow().position);
        }
        // Sem superfície de tela cheia, perder o foco é o único sinal de um clique fora.
        self.clique_livre.set(self.janela_comum);
        if !self.janela_comum {
            self.window.set_keyboard_mode(KeyboardMode::Exclusive);
            self.liberar_entrada();
        }
        self.window.present();
        self.focar(app);
        self.acompanhar_desfoque(app);
        // O COSMIC move o foco ao receber um clique, e é isso que permite soltar a entrada.
        if app.ambiente == Ambiente::Cosmic && app.cfg.borrow().position == Position::BottomRight {
            self.agendar_clique_livre();
        }
    }

    pub fn fechar(&self) {
        self.clique_livre.set(false);
        self.window.set_visible(false);
        if !self.search.text().is_empty() {
            self.search.set_text("");
        }
    }
}
