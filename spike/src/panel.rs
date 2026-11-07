//! Painel de prova: layer surface, foco exclusivo, copia e cola no app de origem.
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gtk4::prelude::*;
use gtk4::{
    gdk, gio, glib, Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation,
};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

#[derive(Clone)]
pub struct Opts {
    pub auto: bool,
    pub dc: bool,
    pub full: bool,
    pub paste: bool,
    pub shift: bool,
    pub size: usize,
    pub delay_ms: u64,
    pub hold_ms: u64,
}

/// Sem foco nesse tempo, o painel não recebeu o teclado.
const ESPERA_FOCO: Duration = Duration::from_secs(3);

const CSS: &str = "
window.supercola-full { background: transparent; }
window.supercola-card { background: transparent; }
.card { background: rgba(32,32,32,0.92); color: white; border-radius: 8px; padding: 16px; }
.card label { margin-bottom: 8px; }
";

fn iniciar(app: &Application, opts: &Opts) {
    // Sem layer-shell seria uma janela comum, e o teste não provaria nada sobre o painel.
    if !gtk4_layer_shell::is_supported() {
        println!("ERRO painel sem layer-shell: confira GDK_BACKEND");
        app.quit();
        return;
    }
    let janela = ApplicationWindow::new(app);
    janela.init_layer_shell();
    janela.set_layer(Layer::Overlay);
    janela.set_keyboard_mode(KeyboardMode::Exclusive);
    janela.set_namespace(Some("supercola-spike"));
    janela.set_exclusive_zone(-1);
    let ancoras: &[Edge] = if opts.full {
        &[Edge::Top, Edge::Bottom, Edge::Left, Edge::Right]
    } else {
        &[Edge::Bottom, Edge::Right]
    };
    for borda in ancoras {
        janela.set_anchor(*borda, true);
    }
    janela.add_css_class(if opts.full {
        "supercola-full"
    } else {
        "supercola-card"
    });

    let provedor = gtk4::CssProvider::new();
    provedor.load_from_string(CSS);
    let display = gdk::Display::default().expect("sem display");
    gtk4::style_context_add_provider_for_display(
        &display,
        &provedor,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let cartao = GtkBox::new(Orientation::Vertical, 8);
    cartao.add_css_class("card");
    cartao.set_size_request(360, -1);
    cartao.set_halign(gtk4::Align::End);
    cartao.set_valign(gtk4::Align::End);
    cartao.set_margin_end(16);
    cartao.set_margin_bottom(16);
    let via = if opts.dc { "data-control" } else { "GDK" };
    cartao.append(&Label::new(Some(&format!(
        "supercola-spike: copiar via {via}"
    ))));
    let botao = Button::with_label("Copiar e colar (Enter)");
    cartao.append(&botao);
    janela.set_child(Some(&cartao));

    let o = opts.clone();
    let j = janela.clone();
    let a = app.clone();
    botao.connect_clicked(move |_| {
        let texto = Arc::new(crate::payload::make(o.size));
        if o.dc {
            let t = texto.clone();
            std::thread::spawn(move || {
                if let Err(e) = crate::dc::serve(t) {
                    eprintln!("ERRO data-control: {e}");
                }
            });
        } else {
            gdk::Display::default()
                .expect("sem display")
                .clipboard()
                .set_text(&texto);
        }
        println!(
            "COPIADO via={} len={} hash={:016x}",
            if o.dc { "dc" } else { "gdk" },
            texto.len(),
            crate::payload::hash(texto.as_bytes())
        );
        j.set_visible(false);

        if o.paste {
            let shift = o.shift;
            glib::timeout_add_local_once(Duration::from_millis(o.delay_ms), move || {
                std::thread::spawn(move || match supercola_wayland::keyboard::paste(shift) {
                    Ok(()) => println!("INJETADO shift={shift}"),
                    Err(e) => eprintln!("ERRO teclado virtual: {e}"),
                });
            });
        }
        let a = a.clone();
        glib::timeout_add_local_once(Duration::from_millis(o.hold_ms), move || a.quit());
    });

    let chaves = gtk4::EventControllerKey::new();
    let a = app.clone();
    chaves.connect_key_pressed(move |_, tecla, _, _| {
        if tecla == gdk::Key::Escape {
            println!("ESC");
            a.quit();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    janela.add_controller(chaves);

    if opts.full {
        let clique = gtk4::GestureClick::new();
        let j = janela.clone();
        let c = cartao.clone();
        let a = app.clone();
        clique.connect_pressed(move |_, _, x, y| {
            let dentro = j
                .pick(x, y, gtk4::PickFlags::DEFAULT)
                .is_some_and(|w| w == *c.upcast_ref::<gtk4::Widget>() || w.is_ancestor(&c));
            if !dentro {
                println!("CLIQUE-FORA");
                a.quit();
            }
        });
        janela.add_controller(clique);
    }

    if opts.auto {
        confirmar_ao_receber_foco(&janela, &botao);
    }

    janela.present();
    botao.grab_focus();
    println!("PAINEL aberto");
}

/// Mede o tempo até o foco e confirma sozinho: o teste não depende de alguém apertar Enter.
fn confirmar_ao_receber_foco(janela: &ApplicationWindow, botao: &Button) {
    let inicio = Instant::now();
    let recebeu = Rc::new(Cell::new(false));

    let (b, r) = (botao.clone(), recebeu.clone());
    janela.connect_is_active_notify(move |j| {
        if j.is_active() && !r.replace(true) {
            println!("FOCO-PAINEL ms={}", inicio.elapsed().as_millis());
            // Na volta do laço: o clique usa o foco que acabou de chegar.
            let b = b.clone();
            glib::idle_add_local_once(move || b.emit_clicked());
        }
    });

    glib::timeout_add_local_once(ESPERA_FOCO, move || {
        if !recebeu.get() {
            println!("SEM-FOCO");
        }
    });
}

pub fn run(opts: Opts) -> crate::R {
    let app = Application::builder()
        .application_id("dev.supercola.SpikePanel")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(move |app| iniciar(app, &opts));
    app.run_with_args::<String>(&[]);
    Ok(())
}
