//! Aba inicial: os itens usados por último em cada aba, com um atalho para ela.
//!
//! O GIF aparece só como atalho: abrir o painel não pode acessar a rede.
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, FlowBox, FlowBoxChild, Label, Orientation, PolicyType,
    ScrolledWindow, SelectionMode,
};
use supercola_core::ipc::Tab;
use supercola_core::lists::{KAOMOJI, SIMBOLOS};

use crate::app::App;
use crate::panel::icone;

/// Sugestões para completar a seção enquanto houver poucos recentes.
const EMOJIS: &[&str] = &[
    "😀", "😂", "😍", "😊", "😎", "😭", "🤔", "👍", "🙏", "🎉", "❤️", "🔥",
];

struct Secao {
    aba: Tab,
    grade: FlowBox,
    limite: usize,
}

pub struct HomeTab {
    pub root: ScrolledWindow,
    secoes: Vec<Secao>,
    atalho_gif: GtkBox,
}

fn cabecalho(app: &Rc<App>, titulo: &str, aba: Tab) -> GtkBox {
    let linha = GtkBox::new(Orientation::Horizontal, 0);
    linha.add_css_class("sc-secao");
    let rotulo = Label::new(Some(titulo));
    rotulo.add_css_class("sc-title");
    rotulo.set_hexpand(true);
    rotulo.set_xalign(0.0);
    linha.append(&rotulo);
    let ir = Button::new();
    ir.add_css_class("sc-botao-seta");
    ir.set_child(Some(&icone("supercola-chevron-symbolic")));
    ir.set_tooltip_text(Some(titulo));
    ir.set_focus_on_click(false);
    let a = app.clone();
    ir.connect_clicked(move |_| a.ir_para_aba(aba));
    linha.append(&ir);
    linha
}

fn sugestoes(aba: Tab) -> Vec<&'static str> {
    match aba {
        Tab::Emoji => EMOJIS.to_vec(),
        Tab::Kaomoji => KAOMOJI.first().map_or(&[][..], |g| g.itens).to_vec(),
        Tab::Symbols => SIMBOLOS.first().map_or(&[][..], |g| g.itens).to_vec(),
        _ => Vec::new(),
    }
}

/// Primeiro os recentes, depois as sugestões, sem repetir.
fn itens(recentes: &[String], extras: &[&str], limite: usize) -> Vec<String> {
    let mut saida: Vec<String> = recentes.iter().take(limite).cloned().collect();
    for extra in extras {
        if saida.len() >= limite {
            break;
        }
        if !saida.iter().any(|s| s == extra) {
            saida.push((*extra).to_string());
        }
    }
    saida
}

fn celula(aba: Tab, texto: &str) -> FlowBoxChild {
    let rotulo = Label::new(Some(texto));
    rotulo.add_css_class("sc-cell");
    rotulo.add_css_class(match aba {
        Tab::Emoji => "sc-cell-emoji",
        Tab::Kaomoji => "sc-cell-kaomoji",
        _ => "sc-cell-symbol",
    });
    rotulo.set_tooltip_text(Some(texto));
    if aba == Tab::Kaomoji {
        rotulo.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        rotulo.set_max_width_chars(12);
    }
    let filho = FlowBoxChild::new();
    filho.set_child(Some(&rotulo));
    filho
}

impl HomeTab {
    pub fn new(app: &Rc<App>) -> Self {
        let caixa = GtkBox::new(Orientation::Vertical, 0);
        caixa.set_valign(Align::Start);
        caixa.set_margin_bottom(8);

        let mut secoes = Vec::new();
        for (aba, chave, colunas, limite) in [
            (Tab::Emoji, "tab_emoji", 6, 12),
            (Tab::Kaomoji, "tab_kaomoji", 3, 6),
            (Tab::Symbols, "tab_symbols", 6, 6),
        ] {
            caixa.append(&cabecalho(app, app.tr(chave), aba));
            let grade = FlowBox::new();
            grade.add_css_class("sc-grade-inicio");
            grade.set_selection_mode(SelectionMode::None);
            grade.set_min_children_per_line(colunas);
            grade.set_max_children_per_line(colunas);
            grade.set_homogeneous(true);
            grade.set_activate_on_single_click(true);
            grade.set_margin_start(16);
            grade.set_margin_end(16);
            let a = app.clone();
            grade.connect_child_activated(move |_, filho| {
                let texto = filho.child().and_downcast::<Label>().map(|l| l.text());
                if let Some(texto) = texto {
                    a.escolher(aba, &texto);
                }
            });
            caixa.append(&grade);
            secoes.push(Secao { aba, grade, limite });
        }
        let atalho_gif = cabecalho(app, app.tr("tab_gif"), Tab::Gif);
        caixa.append(&atalho_gif);

        let root = ScrolledWindow::new();
        root.set_hscrollbar_policy(PolicyType::Never);
        root.set_child(Some(&caixa));

        Self {
            root,
            secoes,
            atalho_gif,
        }
    }

    pub fn atualizar(&self, app: &Rc<App>) {
        for secao in &self.secoes {
            secao.grade.remove_all();
            let recentes = app.recent.borrow().list(secao.aba.name()).to_vec();
            for texto in itens(&recentes, &sugestoes(secao.aba), secao.limite) {
                secao.grade.append(&celula(secao.aba, &texto));
            }
        }
        let tem_chave = !app.cfg.borrow().gif_api_key.trim().is_empty();
        self.atalho_gif.set_visible(tem_chave);
    }

    pub fn focar(&self) -> bool {
        self.secoes
            .first()
            .and_then(|s| s.grade.child_at_index(0))
            .is_some_and(|c| c.grab_focus())
    }

    pub fn ativar_selecionado(&self) {
        let foco = self
            .root
            .root()
            .and_then(|r| gtk4::prelude::RootExt::focus(&r))
            .and_then(|w| w.downcast::<FlowBoxChild>().ok());
        if let Some(filho) = foco {
            filho.emit_activate();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recentes_vem_primeiro_sem_repetir_nem_passar_do_limite() {
        let recentes = ["b".to_string()];
        assert_eq!(itens(&recentes, &["a", "b", "c"], 3), ["b", "a", "c"]);
        assert_eq!(itens(&recentes, &["a", "b", "c"], 1), ["b"]);
    }
}
