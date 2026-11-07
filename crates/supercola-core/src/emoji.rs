//! Emojis com nomes e palavras-chave em português e inglês (dados do emojibase).
use serde::Deserialize;

use crate::i18n::Lang;
use crate::search::{fold, Query};

const DADOS: &str = include_str!("../../../data/emoji.json");

#[derive(Deserialize)]
struct Bruto {
    e: String,
    g: u8,
    pt: String,
    en: String,
    #[serde(default)]
    s: Vec<String>,
}

pub struct Emoji {
    pub text: String,
    pub group: u8,
    /// Nome principal, para acessibilidade e dicas.
    pub label: String,
    busca: String,
    /// As cinco variantes de tom de pele, se houver.
    pub skins: Vec<String>,
}

impl Emoji {
    /// Aplica o tom escolhido (0 = sem tom, 1 a 5 = do mais claro ao mais escuro).
    pub fn with_tone(&self, tom: usize) -> &str {
        match tom {
            1..=5 if self.skins.len() == 5 => &self.skins[tom - 1],
            _ => &self.text,
        }
    }
}

pub fn all(lang: Lang) -> Vec<Emoji> {
    let brutos: Vec<Bruto> = serde_json::from_str(DADOS).expect("data/emoji.json inválido");
    brutos
        .into_iter()
        .map(|b| {
            // Sem tradução, cai no inglês. A busca aceita sempre as duas línguas.
            let principal = if lang == Lang::Pt && !b.pt.is_empty() {
                &b.pt
            } else {
                &b.en
            };
            Emoji {
                label: principal.clone(),
                busca: fold(&format!("{} {}", b.pt, b.en)),
                text: b.e,
                group: b.g,
                skins: b.s,
            }
        })
        .collect()
}

pub fn search<'a>(emojis: &'a [Emoji], query: &str) -> Vec<&'a Emoji> {
    let q = Query::new(query);
    emojis.iter().filter(|e| q.matches(&e.busca)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dados_carregam_e_nao_trazem_componentes() {
        let e = all(Lang::Pt);
        assert!(e.len() > 1500, "{}", e.len());
        assert!(e.iter().all(|x| x.group != 2));
        assert!(e.iter().all(|x| x.skins.is_empty() || x.skins.len() == 5));
    }

    #[test]
    fn busca_em_portugues_sem_acento() {
        let e = all(Lang::Pt);
        let r = search(&e, "coracao");
        assert!(
            r.iter().any(|x| x.text == "❤️" || x.text == "❤"),
            "{}",
            r.len()
        );
    }

    #[test]
    fn busca_em_ingles_tambem_funciona() {
        let e = all(Lang::Pt);
        assert!(!search(&e, "heart").is_empty());
    }

    #[test]
    fn tom_de_pele() {
        let e = all(Lang::Pt);
        let aceno = e.iter().find(|x| x.text.starts_with('👋')).unwrap();
        assert_eq!(aceno.with_tone(0), aceno.text);
        assert_ne!(aceno.with_tone(3), aceno.text);
    }
}
