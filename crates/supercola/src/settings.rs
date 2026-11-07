//! Janela de configurações.
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, DropDown, Label, Orientation, PasswordEntry, PolicyType,
    ScrolledWindow, SpinButton, Switch, TextView, Window,
};
use supercola_core::config::{Config, GifProvider, Language, Position, ThemeMode, WriteBackend};
use supercola_core::i18n::Lang;

use crate::app::App;

fn secao(caixa: &GtkBox, titulo: &str, sub: Option<&str>, controle: &impl IsA<gtk4::Widget>) {
    let linha = GtkBox::new(Orientation::Horizontal, 16);
    let textos = GtkBox::new(Orientation::Vertical, 2);
    textos.set_hexpand(true);
    let t = Label::new(Some(titulo));
    t.set_xalign(0.0);
    textos.append(&t);
    if let Some(s) = sub {
        let l = Label::new(Some(s));
        l.set_xalign(0.0);
        l.set_wrap(true);
        l.add_css_class("dim-label");
        textos.append(&l);
    }
    linha.append(&textos);
    controle.set_valign(Align::Center);
    linha.append(controle);
    caixa.append(&linha);
}

fn lista_editavel(linhas: &[String]) -> (ScrolledWindow, TextView) {
    let v = TextView::new();
    v.set_monospace(true);
    v.buffer().set_text(&linhas.join("\n"));
    let r = ScrolledWindow::new();
    r.set_hscrollbar_policy(PolicyType::Automatic);
    r.set_min_content_height(90);
    r.set_has_frame(true);
    r.set_child(Some(&v));
    (r, v)
}

fn linhas(v: &TextView) -> Vec<String> {
    let b = v.buffer();
    b.text(&b.start_iter(), &b.end_iter(), false)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

pub fn abrir(app: &Rc<App>) {
    if let Some(j) = app.janela_config.borrow().as_ref() {
        j.present();
        return;
    }
    let tr = |c: &str| app.tr(c);
    let cfg = app.cfg.borrow().clone();

    let janela = Window::builder()
        .application(&app.gtk_app)
        .title(tr("settings"))
        .default_width(560)
        .default_height(680)
        .build();

    let caixa = GtkBox::new(Orientation::Vertical, 14);
    for m in [
        GtkBox::set_margin_top as fn(&GtkBox, i32),
        GtkBox::set_margin_bottom,
        GtkBox::set_margin_start,
        GtkBox::set_margin_end,
    ] {
        m(&caixa, 20);
    }

    let historico = Switch::builder().active(cfg.history_enabled).build();
    secao(
        &caixa,
        tr("set_history"),
        Some(tr("set_history_sub")),
        &historico,
    );
    let colar = Switch::builder().active(cfg.auto_paste).build();
    secao(
        &caixa,
        tr("set_autopaste"),
        Some(tr("set_autopaste_sub")),
        &colar,
    );
    let espera = SpinButton::with_range(0.0, 2000.0, 10.0);
    espera.set_value(cfg.paste_delay_ms as f64);
    secao(&caixa, tr("set_delay"), None, &espera);

    let posicao = DropDown::from_strings(&[tr("pos_corner"), tr("pos_pointer")]);
    posicao.set_selected(u32::from(cfg.position == Position::NearPointer));
    secao(&caixa, tr("set_position"), None, &posicao);

    let tema = DropDown::from_strings(&[tr("theme_system"), tr("theme_light"), tr("theme_dark")]);
    tema.set_selected(match cfg.theme {
        ThemeMode::System => 0,
        ThemeMode::Light => 1,
        ThemeMode::Dark => 2,
    });
    secao(&caixa, tr("set_theme"), None, &tema);

    let idioma = DropDown::from_strings(&[tr("lang_auto"), "Português", "English"]);
    idioma.set_selected(match cfg.language {
        Language::Auto => 0,
        Language::Pt => 1,
        Language::En => 2,
    });
    secao(&caixa, tr("set_language"), None, &idioma);

    let backend = DropDown::from_strings(&[tr("backend_dc"), tr("backend_gdk")]);
    backend.set_selected(u32::from(cfg.write_backend == WriteBackend::Gdk));
    secao(&caixa, tr("set_backend"), None, &backend);

    let provedor = DropDown::from_strings(&["KLIPY", "GIPHY"]);
    provedor.set_selected(u32::from(cfg.gif_provider == GifProvider::Giphy));
    secao(&caixa, tr("set_gif_provider"), None, &provedor);
    let chave = PasswordEntry::new();
    chave.set_show_peek_icon(true);
    chave.set_text(&cfg.gif_api_key);
    chave.set_width_chars(24);
    secao(&caixa, tr("set_gif_key"), None, &chave);

    let atalho = Button::with_label(tr("shortcut_install"));
    let resultado = Label::new(None);
    resultado.set_wrap(true);
    resultado.set_xalign(0.0);
    resultado.add_css_class("dim-label");
    let r = resultado.clone();
    atalho.connect_clicked(move |_| {
        r.set_text(&crate::shortcut::install().unwrap_or_else(|e| e));
    });
    secao(&caixa, tr("set_shortcut"), None, &atalho);
    caixa.append(&resultado);

    let rotulo = |t: &str, s: &str| {
        let l = Label::new(Some(&format!("{t}\n{s}")));
        l.set_xalign(0.0);
        l.set_wrap(true);
        l
    };
    caixa.append(&rotulo(tr("set_terminals"), tr("set_terminals_sub")));
    let (r_term, v_term) = lista_editavel(&cfg.terminals);
    caixa.append(&r_term);
    caixa.append(&rotulo(tr("set_excluded"), tr("set_excluded_sub")));
    let (r_excl, v_excl) = lista_editavel(&cfg.excluded_apps);
    caixa.append(&r_excl);

    let salvar = Button::with_label(tr("save"));
    salvar.add_css_class("suggested-action");
    salvar.set_halign(Align::End);
    caixa.append(&salvar);

    let a = app.clone();
    let j = janela.clone();
    salvar.connect_clicked(move |_| {
        let novo = Config {
            history_enabled: historico.is_active(),
            auto_paste: colar.is_active(),
            paste_delay_ms: espera.value() as u64,
            position: if posicao.selected() == 1 {
                Position::NearPointer
            } else {
                Position::BottomRight
            },
            write_backend: if backend.selected() == 1 {
                WriteBackend::Gdk
            } else {
                WriteBackend::DataControl
            },
            theme: match tema.selected() {
                1 => ThemeMode::Light,
                2 => ThemeMode::Dark,
                _ => ThemeMode::System,
            },
            language: match idioma.selected() {
                1 => Language::Pt,
                2 => Language::En,
                _ => Language::Auto,
            },
            terminals: linhas(&v_term),
            excluded_apps: linhas(&v_excl),
            gif_provider: if provedor.selected() == 1 {
                GifProvider::Giphy
            } else {
                GifProvider::Klipy
            },
            gif_api_key: chave.text().trim().to_string(),
        };
        if let Err(e) = novo.save() {
            eprintln!("supercola: não consegui salvar a configuração: {e}");
        }
        a.lang.set(Lang::resolve(novo.language));
        *a.cfg.borrow_mut() = novo;
        a.reaplicar_tema();
        a.recriar_ui();
        j.close();
    });

    let a = app.clone();
    janela.connect_close_request(move |_| {
        *a.janela_config.borrow_mut() = None;
        gtk4::glib::Propagation::Proceed
    });

    let rolagem = ScrolledWindow::new();
    rolagem.set_hscrollbar_policy(PolicyType::Never);
    rolagem.set_child(Some(&caixa));
    janela.set_child(Some(&rolagem));
    *app.janela_config.borrow_mut() = Some(janela.clone());
    janela.present();
}
