//! Textos da interface em português e inglês.
use crate::config::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Pt,
    En,
}

impl Lang {
    pub fn resolve(preferida: Language) -> Self {
        match preferida {
            Language::Pt => Self::Pt,
            Language::En => Self::En,
            Language::Auto => Self::from_locale(
                &["LC_ALL", "LC_MESSAGES", "LANG"]
                    .iter()
                    .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
                    .unwrap_or_default(),
            ),
        }
    }

    fn from_locale(locale: &str) -> Self {
        if locale.to_ascii_lowercase().starts_with("pt") {
            Self::Pt
        } else {
            Self::En
        }
    }

    pub fn tr(self, chave: &str) -> &'static str {
        match TEXTOS.iter().find(|(c, _, _)| *c == chave) {
            Some((_, pt, en)) => {
                if self == Self::Pt {
                    pt
                } else {
                    en
                }
            }
            None => {
                debug_assert!(false, "texto sem tradução: {chave}");
                ""
            }
        }
    }
}

#[rustfmt::skip]
const TEXTOS: &[(&str, &str, &str)] = &[
    ("clipboard_history", "Área de transferência", "Clipboard"),
    ("clear_all", "Limpar tudo", "Clear all"),
    ("search", "Pesquisar", "Search"),
    ("search_emoji", "Pesquisar emojis", "Search emojis"),
    ("search_gif", "Pesquisar GIFs", "Search GIFs"),
    ("search_results", "Resultados da pesquisa", "Search results"),
    ("close", "Fechar", "Close"),
    ("back", "Voltar", "Back"),
    ("tab_home", "Usados recentemente", "Recently used"),
    ("tab_emoji", "Emoji", "Emoji"),
    ("tab_gif", "GIF", "GIF"),
    ("tab_kaomoji", "Kaomoji", "Kaomoji"),
    ("tab_symbols", "Símbolos", "Symbols"),
    ("tab_history", "Área de transferência", "Clipboard"),
    ("empty_title", "Nada aqui", "Nothing here"),
    ("empty_body", "Você verá o histórico da área de transferência aqui depois de copiar algo.", "You'll see your clipboard history here once you've copied something."),
    ("off_title", "Vamos começar", "Let's get started"),
    ("off_body", "Ative o histórico da área de transferência para copiar e ver vários itens.", "Turn on clipboard history to copy and view multiple items."),
    ("turn_on", "Ativar", "Turn on"),
    ("no_results", "Nada encontrado", "Nothing found"),
    ("pin", "Fixar", "Pin"),
    ("unpin", "Desafixar", "Unpin"),
    ("delete", "Excluir", "Delete"),
    ("paste_text", "Colar como texto", "Paste as text"),
    ("see_more", "Ver mais", "See more"),
    ("image", "Imagem", "Image"),
    ("recent", "Recentes", "Recently used"),
    ("skin_tone", "Tom de pele", "Skin tone"),
    ("gif_setup_title", "Ative a aba de GIF", "Turn on the GIF tab"),
    ("gif_setup_body", "Informe uma chave de API nas configurações. As buscas desta aba serão enviadas ao serviço escolhido.", "Enter an API key in settings. Searches on this tab are sent to the service you choose."),
    ("gif_error", "Não consegui buscar GIFs", "Couldn't load GIFs"),
    ("open_settings", "Abrir configurações", "Open settings"),
    ("settings", "Configurações do Super Cola", "Super Cola settings"),
    ("set_history", "Guardar histórico", "Keep history"),
    ("set_history_sub", "Itens comuns ficam só na memória e somem ao reiniciar.", "Regular items stay in memory and vanish on restart."),
    ("set_autopaste", "Colar automaticamente", "Paste automatically"),
    ("set_autopaste_sub", "Sem isto, o item só é copiado.", "Without this, the item is only copied."),
    ("set_delay", "Espera antes de colar (ms)", "Delay before pasting (ms)"),
    ("set_position", "Posição do painel", "Panel position"),
    ("pos_corner", "Canto inferior direito", "Bottom right corner"),
    ("pos_pointer", "Perto do mouse", "Near the pointer"),
    ("set_theme", "Tema", "Theme"),
    ("theme_system", "Do sistema", "System"),
    ("theme_light", "Claro", "Light"),
    ("theme_dark", "Escuro", "Dark"),
    ("set_language", "Idioma", "Language"),
    ("lang_auto", "Do sistema", "System"),
    ("set_backend", "Gravar no clipboard via", "Write to the clipboard via"),
    ("backend_dc", "data-control (recomendado)", "data-control (recommended)"),
    ("backend_gdk", "Cópia comum", "Regular copy"),
    ("set_terminals", "Terminais (um por linha)", "Terminals (one per line)"),
    ("set_terminals_sub", "Estes apps recebem Ctrl+Shift+V.", "These apps receive Ctrl+Shift+V."),
    ("set_excluded", "Apps ignorados (um por linha)", "Ignored apps (one per line)"),
    ("set_excluded_sub", "O que for copiado neles não entra no histórico.", "Whatever is copied in them stays out of the history."),
    ("set_gif_provider", "Serviço de GIF", "GIF service"),
    ("set_gif_key", "Chave de API de GIF", "GIF API key"),
    ("set_shortcut", "Atalho Super+V", "Super+V shortcut"),
    ("shortcut_install", "Registrar", "Register"),
    ("save", "Salvar", "Save"),
    ("restart_needed", "Idioma e tema aparecem na próxima abertura do painel.", "Language and theme apply the next time the panel opens."),
    ("emoji_rostos", "Rostos e animais", "Smiley faces and animals"),
    ("emoji_pessoas", "Pessoas", "People"),
    ("emoji_objetos", "Celebrações e objetos", "Celebrations and objects"),
    ("emoji_comida", "Comida e plantas", "Food and plants"),
    ("emoji_lugares", "Transporte e lugares", "Transportation and places"),
    ("emoji_simbolos", "Símbolos", "Symbols"),
    ("kao_classicos", "Emoticons ASCII clássicos", "Classic ASCII emoticons"),
    ("kao_feliz", "Feliz", "Happy"),
    ("kao_triste", "Triste", "Sad"),
    ("kao_bravo", "Bravo", "Angry"),
    ("kao_amor", "Amor", "Love"),
    ("kao_outros", "Outros", "Other"),
    ("sym_pontuacao", "Pontuação geral", "General punctuation"),
    ("sym_moedas", "Símbolos de moeda", "Currency symbols"),
    ("sym_latinos", "Símbolos latinos", "Latin symbols"),
    ("sym_geometricos", "Símbolos geométricos", "Geometric symbols"),
    ("sym_matematicos", "Símbolos matemáticos", "Math symbols"),
    ("sym_suplementares", "Símbolos suplementares", "Supplemental symbols"),
    ("sym_idiomas", "Símbolos de idiomas", "Language symbols"),
    ("gif_trending", "Em alta", "Trending"),
    ("gif_powered", "Fornecido por", "Powered By"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escolhe_idioma_pelo_locale() {
        assert_eq!(Lang::from_locale("pt_BR.UTF-8"), Lang::Pt);
        assert_eq!(Lang::from_locale("en_US.UTF-8"), Lang::En);
        assert_eq!(Lang::from_locale(""), Lang::En);
    }

    #[test]
    fn chaves_sao_unicas_e_preenchidas() {
        for (i, (c, pt, en)) in TEXTOS.iter().enumerate() {
            assert!(!pt.is_empty() && !en.is_empty(), "{c}");
            assert!(!TEXTOS[..i].iter().any(|(o, _, _)| o == c), "{c}");
        }
    }
}
