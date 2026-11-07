//! Tema claro/escuro e cor de destaque, lidos do portal `Settings`.
use gtk4::glib::Variant;
use gtk4::prelude::*;
use gtk4::{gdk, gio};
use supercola_core::config::ThemeMode;

const CSS: &str = include_str!("../../../data/style.css");
const NAMESPACE: &str = "org.freedesktop.appearance";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aparencia {
    pub escuro: bool,
    pub destaque: Option<(f64, f64, f64)>,
}

fn proxy() -> Option<gio::DBusProxy> {
    gio::DBusProxy::for_bus_sync(
        gio::BusType::Session,
        gio::DBusProxyFlags::NONE,
        None,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Settings",
        gio::Cancellable::NONE,
    )
    .ok()
}

fn ler(proxy: &gio::DBusProxy, chave: &str) -> Option<Variant> {
    let args = (NAMESPACE, chave).to_variant();
    let r = proxy
        .call_sync(
            "ReadOne",
            Some(&args),
            gio::DBusCallFlags::NONE,
            500,
            gio::Cancellable::NONE,
        )
        .ok()?;
    // Resposta `(v)`: desembrulha a tupla e o variant.
    r.child_value(0).as_variant()
}

fn destaque(v: &Variant) -> Option<(f64, f64, f64)> {
    let (r, g, b) = v.get::<(f64, f64, f64)>()?;
    // Fora de 0..1 significa "sem preferência".
    [r, g, b]
        .iter()
        .all(|c| (0.0..=1.0).contains(c))
        .then_some((r, g, b))
}

pub fn ler_sistema() -> Aparencia {
    let Some(p) = proxy() else {
        return Aparencia {
            escuro: false,
            destaque: None,
        };
    };
    Aparencia {
        escuro: ler(&p, "color-scheme").and_then(|v| v.get::<u32>()) == Some(1),
        destaque: ler(&p, "accent-color").as_ref().and_then(destaque),
    }
}

pub fn acompanhar(aviso: impl Fn() + 'static) {
    let Some(p) = proxy() else { return };
    p.connect_local("g-signal", false, move |args| {
        if args.get(2).and_then(|n| n.get::<String>().ok()).as_deref() == Some("SettingChanged") {
            aviso();
        }
        None
    });
    // Mantém o proxy vivo pelo resto do processo.
    std::mem::forget(p);
}

fn cor(c: (f64, f64, f64)) -> String {
    let b = |v: f64| (v * 255.0).round() as u8;
    format!("rgb({},{},{})", b(c.0), b(c.1), b(c.2))
}

/// Tokens do Fluent/WinUI 3 para cada tema. Com desfoque, o painel deixa passar a tela.
fn tokens(a: Aparencia, translucido: bool) -> String {
    // #0067C0, medido no painel do Windows 11, e #60CDFF.
    let (claro, escuro) = ((0.0, 0.404, 0.753), (0.376, 0.804, 1.0));
    let acento = cor(a.destaque.unwrap_or(if a.escuro { escuro } else { claro }));
    let base = if a.escuro {
        "@define-color sc_card rgba(255,255,255,0.0605);
         @define-color sc_card_hover rgba(255,255,255,0.0837);
         @define-color sc_card_press rgba(255,255,255,0.0326);
         @define-color sc_stroke rgba(255,255,255,0.0698);
         @define-color sc_panel_stroke rgba(0,0,0,0.4);
         @define-color sc_text rgb(255,255,255);
         @define-color sc_text2 rgba(255,255,255,0.786);
         @define-color sc_text3 rgba(255,255,255,0.544);
         @define-color sc_subtle rgba(255,255,255,0.0605);
         @define-color sc_on_accent rgb(0,0,0);"
    } else {
        "@define-color sc_card rgba(255,255,255,0.7);
         @define-color sc_card_hover rgba(249,249,249,0.5);
         @define-color sc_card_press rgba(249,249,249,0.3);
         @define-color sc_stroke rgba(0,0,0,0.0578);
         @define-color sc_panel_stroke rgba(0,0,0,0.1);
         @define-color sc_text rgba(0,0,0,0.894);
         @define-color sc_text2 rgba(0,0,0,0.62);
         @define-color sc_text3 rgba(0,0,0,0.446);
         @define-color sc_subtle rgba(0,0,0,0.0373);
         @define-color sc_on_accent rgb(255,255,255);"
    };
    let cinza = if a.escuro { 44 } else { 243 };
    let alfa = match (translucido, a.escuro) {
        (true, false) => 0.85,
        (true, true) => 0.8,
        (false, _) => 0.96,
    };
    format!(
        "{base}\n@define-color sc_panel rgba({cinza},{cinza},{cinza},{alfa});\n\
         @define-color sc_accent {acento};\n"
    )
}

pub struct Tema {
    provedor: gtk4::CssProvider,
}

impl Tema {
    pub fn novo() -> Self {
        let provedor = gtk4::CssProvider::new();
        let display = gdk::Display::default().expect("sem display");
        gtk4::style_context_add_provider_for_display(
            &display,
            &provedor,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
        );
        if let Some(gs) = gtk4::Settings::default() {
            gs.set_gtk_enable_animations(false);
        }
        Self { provedor }
    }

    pub fn aplicar(&self, modo: ThemeMode, translucido: bool) {
        let sistema = ler_sistema();
        let a = match modo {
            ThemeMode::System => sistema,
            ThemeMode::Light => Aparencia {
                escuro: false,
                ..sistema
            },
            ThemeMode::Dark => Aparencia {
                escuro: true,
                ..sistema
            },
        };
        let filtro = if a.escuro {
            "brightness(0) invert(1) opacity(0.95)"
        } else {
            "brightness(0) opacity(0.9)"
        };
        self.provedor.load_from_string(&format!(
            "{}{}",
            tokens(a, translucido),
            CSS.replace("@ICON_FILTER@", filtro)
        ));
        if let Some(gs) = gtk4::Settings::default() {
            gs.set_gtk_application_prefer_dark_theme(a.escuro);
        }
    }
}
