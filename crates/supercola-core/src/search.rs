//! Busca que ignora acentos e caixa.
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

pub fn fold(text: &str) -> String {
    text.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Todas as palavras precisam aparecer, em qualquer ordem.
pub struct Query(Vec<String>);

impl Query {
    pub fn new(text: &str) -> Self {
        Self(fold(text).split_whitespace().map(String::from).collect())
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn matches(&self, text: &str) -> bool {
        let alvo = fold(text);
        self.0.iter().all(|p| alvo.contains(p.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dobra_acentos() {
        assert_eq!(fold("Coração ÀS"), "coracao as");
    }

    #[test]
    fn exige_todas_as_palavras() {
        let q = Query::new("rosto feliz");
        assert!(q.matches("Feliz, um rosto"));
        assert!(!q.matches("rosto triste"));
    }
}
