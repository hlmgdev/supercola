//! Estado do daemon e o fluxo de copiar e colar.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gtk4::prelude::*;
use gtk4::{gdk, glib, Application};
use supercola_core::config::{Config, WriteBackend};
use supercola_core::filter;
use supercola_core::history::{Added, History};
use supercola_core::i18n::Lang;
use supercola_core::ipc::{Command, Tab};
use supercola_core::model::{self, Flavor};
use supercola_core::recent::Recent;
use supercola_core::store::Store;
use supercola_core::xdg;
use supercola_wayland::clipboard::{self, Event, Handle};
use supercola_wayland::focus::Focus;

use crate::blur::Desfoque;
use crate::panel::Ui;
use crate::theme::{self, Tema};

pub struct App {
    pub gtk_app: Application,
    pub cfg: RefCell<Config>,
    pub lang: Cell<Lang>,
    pub history: RefCell<History>,
    pub recent: RefCell<Recent>,
    pub texturas: RefCell<HashMap<u64, gdk::Texture>>,
    pub janela_config: RefCell<Option<gtk4::Window>>,
    /// O compositor é o COSMIC, que move o foco ao receber um clique.
    pub cosmic: bool,
    /// Um popover aberto tira o foco da janela, sem que isso seja um clique fora.
    pub popover_aberto: Cell<bool>,
    pub desfoque: Option<Desfoque>,
    ui: RefCell<Option<Rc<Ui>>>,
    store: RefCell<Option<Store>>,
    clip: Option<Handle>,
    focus: Option<Focus>,
    alvo: RefCell<Option<String>>,
    tema: Tema,
}

pub fn agora() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl App {
    pub fn tr(&self, chave: &str) -> &'static str {
        self.lang.get().tr(chave)
    }

    pub fn start(gtk_app: &Application) {
        let pedidos = match crate::ipc::listen() {
            Ok(rx) => rx,
            Err(e) => {
                eprintln!("supercola: {e}");
                gtk_app.quit();
                return;
            }
        };

        let (config, erro) = Config::load();
        if let Some(e) = erro {
            eprintln!("supercola: config inválida, usando padrões: {e}");
        }
        let lang = Lang::resolve(config.language);

        let store = Store::open(&xdg::data_dir().join("pinned.db"))
            .map_err(|e| eprintln!("supercola: sem persistência de fixados: {e}"))
            .ok();
        let fixados = store
            .as_ref()
            .and_then(|s| s.load().ok())
            .unwrap_or_default();

        let (tx, rx) = async_channel::unbounded();
        let sink = Arc::new(move |ev: Event| {
            let _ = tx.send_blocking(ev);
        });
        let clip = match clipboard::spawn(sink) {
            Ok(h) => {
                eprintln!("supercola: clipboard via {}", h.backend);
                Some(h)
            }
            Err(e) => {
                eprintln!("supercola: sem acesso ao clipboard: {e}");
                None
            }
        };
        let focus = match supercola_wayland::focus::spawn() {
            Ok((f, backend)) => {
                eprintln!("supercola: foco via {backend}");
                Some(f)
            }
            Err(e) => {
                eprintln!("supercola: sem detecção de foco: {e}");
                None
            }
        };

        let tema = Tema::novo();
        let desfoque = gdk::Display::default().and_then(|d| Desfoque::novo(&d));
        tema.aplicar(config.theme, desfoque.is_some());
        if let Some(display) = gdk::Display::default() {
            let icones = crate::data_dir().join("icons");
            gtk4::IconTheme::for_display(&display).add_search_path(icones);
        }

        let app = Rc::new(App {
            gtk_app: gtk_app.clone(),
            cfg: RefCell::new(config),
            lang: Cell::new(lang),
            history: RefCell::new(History::with_pinned(fixados)),
            recent: RefCell::new(Recent::load()),
            texturas: RefCell::default(),
            janela_config: RefCell::new(None),
            cosmic: supercola_wayland::probe::globals()
                .is_ok_and(|g| g.iter().any(|(nome, _)| nome.starts_with("zcosmic_"))),
            popover_aberto: Cell::default(),
            desfoque,
            ui: RefCell::new(None),
            store: RefCell::new(store),
            clip,
            focus,
            alvo: RefCell::new(None),
            tema,
        });

        let a = app.clone();
        theme::acompanhar(move || a.reaplicar_tema());

        let a = app.clone();
        glib::spawn_future_local(async move {
            while let Ok(ev) = rx.recv().await {
                a.no_clipboard(ev);
            }
        });

        let a = app.clone();
        glib::spawn_future_local(async move {
            while let Ok(p) = pedidos.recv().await {
                let texto = a.comando(p.comando);
                let _ = p.resposta.send(texto);
            }
        });

        crate::shortcut::first_run();
    }

    pub fn reaplicar_tema(&self) {
        self.tema
            .aplicar(self.cfg.borrow().theme, self.desfoque.is_some());
    }

    fn no_clipboard(self: &Rc<Self>, ev: Event) {
        let Event::Selection(sabores) = ev else {
            return;
        };
        let cfg = self.cfg.borrow();
        if !cfg.history_enabled {
            return;
        }
        let app_em_foco = self.focus.as_ref().and_then(Focus::current);
        if filter::in_list(app_em_foco.as_deref(), &cfg.excluded_apps) {
            return;
        }
        drop(cfg);
        let r = self.history.borrow_mut().add(sabores, agora());
        if let Added::TooLarge = r {
            eprintln!("supercola: item acima de 4 MB ignorado");
        }
        self.ao_mudar_historico();
    }

    pub fn ao_mudar_historico(self: &Rc<Self>) {
        let ui = self.ui.borrow().clone();
        if let Some(ui) = ui {
            if ui.window.is_visible() {
                ui.history.refresh(self, &ui.search.text());
            }
        }
    }

    pub fn salvar_fixados(&self) {
        let fixados = self.history.borrow().pinned();
        if let Some(s) = self.store.borrow_mut().as_mut() {
            if let Err(e) = s.save(&fixados) {
                eprintln!("supercola: não consegui salvar os fixados: {e}");
            }
        }
    }

    fn comando(self: &Rc<Self>, c: Command) -> String {
        match c {
            Command::Toggle(tab) => {
                self.alternar(tab);
                String::new()
            }
            Command::Hide => {
                self.esconder();
                String::new()
            }
            Command::Settings => {
                crate::settings::abrir(self);
                String::new()
            }
            Command::Status => self.status(),
            Command::Quit => {
                self.gtk_app.quit();
                String::new()
            }
        }
    }

    fn status(&self) -> String {
        let h = self.history.borrow();
        let ui = self.ui.borrow();
        let widget = ui
            .as_ref()
            .and_then(|u| gtk4::prelude::GtkWindowExt::focus(&u.window))
            .map_or("-".to_string(), |w| w.type_().name().to_string());
        format!(
            "items={} pinned={} visible={} tab={} search={} widget={} clipboard={} focus={}\n",
            h.items().len(),
            h.pinned().len(),
            ui.as_ref().is_some_and(|u| u.window.is_visible()),
            ui.as_ref().map_or("-", |u| u.tab.get().name()),
            ui.as_ref().is_some_and(|u| u.search.is_visible()),
            widget,
            self.clip.as_ref().map_or("-", |c| c.backend),
            self.focus
                .as_ref()
                .and_then(Focus::current)
                .unwrap_or_else(|| "-".into()),
        )
    }

    fn ui(self: &Rc<Self>) -> Rc<Ui> {
        let existente = self.ui.borrow().clone();
        existente.unwrap_or_else(|| {
            let nova = Ui::new(self);
            *self.ui.borrow_mut() = Some(nova.clone());
            nova
        })
    }

    /// Reconstrói a interface na próxima abertura, depois de mudar idioma ou tema.
    pub fn recriar_ui(&self) {
        self.tirar_desfoque();
        if let Some(ui) = self.ui.borrow_mut().take() {
            ui.window.destroy();
        }
    }

    /// Leva a outra aba com o painel aberto, como fazem os botões do início.
    pub fn ir_para_aba(self: &Rc<Self>, aba: Tab) {
        self.ui().selecionar(self, aba);
    }

    fn alternar(self: &Rc<Self>, tab: Tab) {
        let ui = self.ui();
        if ui.window.is_visible() && ui.tab.get() == tab {
            self.esconder();
            return;
        }
        // Antes de abrir: é o app que vai receber a colagem.
        *self.alvo.borrow_mut() = self.focus.as_ref().and_then(Focus::current);
        ui.abrir(self, tab);
    }

    fn tirar_desfoque(&self) {
        if let Some(d) = &self.desfoque {
            d.remover();
        }
    }

    pub fn esconder(&self) {
        self.tirar_desfoque();
        let ui = self.ui.borrow().clone();
        if let Some(ui) = ui {
            ui.fechar();
        }
    }

    fn gravar_gdk(&self, oferta: &[(String, Arc<Vec<u8>>)]) {
        let Some(display) = gdk::Display::default() else {
            return;
        };
        let provedores: Vec<gdk::ContentProvider> = oferta
            .iter()
            .map(|(mime, dados)| {
                gdk::ContentProvider::for_bytes(mime, &glib::Bytes::from(dados.as_slice()))
            })
            .collect();
        if let Err(e) = display
            .clipboard()
            .set_content(Some(&gdk::ContentProvider::new_union(&provedores)))
        {
            eprintln!("supercola: não consegui gravar no clipboard: {e}");
        }
    }

    /// Grava o conteúdo, fecha o painel e cola no app de origem.
    ///
    /// `transient`: não entra no histórico, e o clipboard anterior volta depois.
    pub fn colar(self: &Rc<Self>, sabores: Vec<Flavor>, transient: bool) {
        let cfg = self.cfg.borrow().clone();
        let anterior = if transient {
            self.history
                .borrow()
                .items()
                .first()
                .map(|c| c.flavors.clone())
        } else {
            None
        };
        let oferta = model::offered(&sabores, transient);
        match (&self.clip, cfg.write_backend) {
            (Some(c), WriteBackend::DataControl) => c.set(oferta),
            _ => self.gravar_gdk(&oferta),
        }
        self.esconder();

        if !cfg.auto_paste {
            return;
        }
        let shift = filter::in_list(self.alvo.borrow().as_deref(), &cfg.terminals);
        let (tx, rx) = async_channel::bounded::<()>(1);
        glib::timeout_add_local_once(Duration::from_millis(cfg.paste_delay_ms), move || {
            std::thread::spawn(move || {
                if let Err(e) = supercola_wayland::keyboard::paste(shift) {
                    eprintln!("supercola: não consegui colar: {e}");
                }
                let _ = tx.send_blocking(());
            });
        });
        let app = self.clone();
        glib::spawn_future_local(async move {
            let _ = rx.recv().await;
            let Some(sabores) = anterior else { return };
            // O app alvo lê o clipboard de forma assíncrona: damos um tempo antes de restaurar.
            glib::timeout_future(Duration::from_millis(400)).await;
            if let Some(c) = &app.clip {
                c.set(model::offered(&sabores, true));
            }
        });
    }

    pub fn colar_item(self: &Rc<Self>, id: u64, so_texto: bool) {
        let clip = self.history.borrow().get(id).cloned();
        let Some(clip) = clip else { return };
        let sabores = if so_texto {
            clip.flavor(model::MIME_TEXT).cloned().into_iter().collect()
        } else {
            clip.flavors.clone()
        };
        self.history.borrow_mut().touch(id, agora());
        self.colar(sabores, false);
    }

    pub fn escolher(self: &Rc<Self>, aba: Tab, texto: &str) {
        {
            let mut r = self.recent.borrow_mut();
            r.push(aba.name(), texto);
            if let Err(e) = r.save() {
                eprintln!("supercola: não consegui salvar recentes: {e}");
            }
        }
        self.colar(vec![Flavor::text(texto)], true);
    }

    pub fn alterna_historico(self: &Rc<Self>, ligado: bool) {
        let cfg = {
            let mut c = self.cfg.borrow_mut();
            c.history_enabled = ligado;
            c.clone()
        };
        if let Err(e) = cfg.save() {
            eprintln!("supercola: não consegui salvar a configuração: {e}");
        }
        self.ao_mudar_historico();
    }
}
