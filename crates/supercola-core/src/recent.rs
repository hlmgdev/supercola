//! Recentes das abas de emoji, kaomoji e símbolos.
use std::collections::BTreeMap;
use std::fs;

use serde::{Deserialize, Serialize};

const MAX: usize = 24;

#[derive(Default, Serialize, Deserialize)]
pub struct Recent(BTreeMap<String, Vec<String>>);

impl Recent {
    fn path() -> std::path::PathBuf {
        crate::xdg::data_dir().join("recent.json")
    }

    pub fn load() -> Self {
        fs::read(Self::path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path();
        if let Some(pai) = path.parent() {
            fs::create_dir_all(pai)?;
        }
        fs::write(path, serde_json::to_vec(self)?)
    }

    pub fn list(&self, tab: &str) -> &[String] {
        self.0.get(tab).map_or(&[], Vec::as_slice)
    }

    pub fn push(&mut self, tab: &str, value: &str) {
        let lista = self.0.entry(tab.to_string()).or_default();
        lista.retain(|v| v != value);
        lista.insert(0, value.to_string());
        lista.truncate(MAX);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mais_recente_primeiro_sem_repetir() {
        let mut r = Recent::default();
        for v in ["a", "b", "a"] {
            r.push("emoji", v);
        }
        assert_eq!(r.list("emoji"), ["a", "b"]);
        assert!(r.list("gif").is_empty());
    }

    #[test]
    fn limita_a_24() {
        let mut r = Recent::default();
        for i in 0..40 {
            r.push("x", &i.to_string());
        }
        assert_eq!(r.list("x").len(), 24);
    }
}
