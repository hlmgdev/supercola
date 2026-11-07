//! Janela-alvo: mede o que chegou colado e informa tamanho e hash.
use gtk4::prelude::*;
use gtk4::{gdk, gio, glib, Application, ApplicationWindow, ScrolledWindow, TextView};

pub fn run() -> crate::R {
    let app = Application::builder()
        .application_id("dev.supercola.SpikeTarget")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(|app| {
        let view = TextView::new();
        // Registra atalhos de colar, para distinguir Ctrl+V de Ctrl+Shift+V.
        let teclas = gtk4::EventControllerKey::new();
        teclas.set_propagation_phase(gtk4::PropagationPhase::Capture);
        teclas.connect_key_pressed(|_, tecla, _, mods| {
            if matches!(tecla.to_lower(), gdk::Key::v)
                && mods.contains(gdk::ModifierType::CONTROL_MASK)
            {
                let shift = mods.contains(gdk::ModifierType::SHIFT_MASK);
                println!("TECLA ctrl{}+v", if shift { "+shift" } else { "" });
            }
            glib::Propagation::Proceed
        });
        view.add_controller(teclas);
        view.buffer().connect_changed(|buffer| {
            let (inicio, fim) = buffer.bounds();
            let texto = buffer.text(&inicio, &fim, true);
            println!(
                "ALVO len={} hash={:016x}",
                texto.len(),
                crate::payload::hash(texto.as_bytes())
            );
        });
        let rolagem = ScrolledWindow::builder().child(&view).build();
        let janela = ApplicationWindow::builder()
            .application(app)
            .title("supercola-spike alvo")
            .default_width(520)
            .default_height(320)
            .child(&rolagem)
            .build();
        janela.connect_is_active_notify(|j| println!("FOCO-ALVO ativo={}", j.is_active()));
        janela.present();
        view.grab_focus();
    });
    app.run_with_args::<String>(&[]);
    Ok(())
}
