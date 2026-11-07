//! Histórico com as regras do Win+V: 25 itens, 4 MB por item, fixados ficam.
use crate::model::{Clip, Flavor};
use crate::search::Query;

pub const MAX_ITEMS: usize = 25;
pub const MAX_ITEM_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum Added {
    New(u64),
    Bumped(u64),
    TooLarge,
    Empty,
}

#[derive(Default)]
pub struct History {
    items: Vec<Clip>, // mais recente primeiro
    next_id: u64,
}

impl History {
    pub fn with_pinned(pinned: Vec<Clip>) -> Self {
        let next_id = pinned.iter().map(|c| c.id).max().map_or(1, |m| m + 1);
        Self {
            items: pinned,
            next_id,
        }
    }

    pub fn items(&self) -> &[Clip] {
        &self.items
    }

    pub fn get(&self, id: u64) -> Option<&Clip> {
        self.items.iter().find(|c| c.id == id)
    }

    pub fn pinned(&self) -> Vec<Clip> {
        self.items.iter().filter(|c| c.pinned).cloned().collect()
    }

    pub fn add(&mut self, flavors: Vec<Flavor>, now: u64) -> Added {
        if flavors.iter().all(|f| f.data.is_empty()) {
            return Added::Empty;
        }
        if flavors.iter().map(|f| f.data.len()).sum::<usize>() > MAX_ITEM_BYTES {
            return Added::TooLarge;
        }
        if let Some(pos) = self.items.iter().position(|c| c.same_content(&flavors)) {
            let mut clip = self.items.remove(pos);
            clip.copied_at = now;
            let id = clip.id;
            self.items.insert(0, clip);
            return Added::Bumped(id);
        }
        self.next_id = self.next_id.max(1);
        let id = self.next_id;
        self.next_id += 1;
        self.items.insert(
            0,
            Clip {
                id,
                flavors,
                pinned: false,
                copied_at: now,
            },
        );
        self.evict();
        Added::New(id)
    }

    fn evict(&mut self) {
        while self.items.len() > MAX_ITEMS {
            let Some(pos) = self.items.iter().rposition(|c| !c.pinned) else {
                return;
            };
            self.items.remove(pos);
        }
    }

    /// Sobe o item para o topo, como acontece ao colar um item antigo.
    pub fn touch(&mut self, id: u64, now: u64) {
        if let Some(pos) = self.items.iter().position(|c| c.id == id) {
            let mut clip = self.items.remove(pos);
            clip.copied_at = now;
            self.items.insert(0, clip);
        }
    }

    pub fn remove(&mut self, id: u64) -> bool {
        let antes = self.items.len();
        self.items.retain(|c| c.id != id);
        self.items.len() != antes
    }

    pub fn toggle_pin(&mut self, id: u64) -> Option<bool> {
        let clip = self.items.iter_mut().find(|c| c.id == id)?;
        clip.pinned = !clip.pinned;
        let pinned = clip.pinned;
        self.evict();
        Some(pinned)
    }

    /// "Limpar tudo": preserva os fixados.
    pub fn clear(&mut self) {
        self.items.retain(|c| c.pinned);
    }

    pub fn search(&self, query: &str) -> Vec<&Clip> {
        let q = Query::new(query);
        self.items
            .iter()
            .filter(|c| q.is_empty() || c.text().is_some_and(|t| q.matches(t)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn txt(s: &str) -> Vec<Flavor> {
        vec![Flavor::text(s)]
    }

    #[test]
    fn mantem_no_maximo_25_e_descarta_o_mais_antigo() {
        let mut h = History::default();
        for i in 0..30 {
            h.add(txt(&format!("item {i}")), i);
        }
        assert_eq!(h.items().len(), 25);
        assert_eq!(h.items()[0].text(), Some("item 29"));
        assert_eq!(h.items()[24].text(), Some("item 5"));
    }

    #[test]
    fn recopiar_sobe_o_item_sem_duplicar() {
        let mut h = History::default();
        let a = h.add(txt("a"), 1);
        h.add(txt("b"), 2);
        let Added::New(id) = a else { panic!() };
        assert_eq!(h.add(txt("a"), 3), Added::Bumped(id));
        assert_eq!(h.items().len(), 2);
        assert_eq!(h.items()[0].text(), Some("a"));
    }

    #[test]
    fn recusa_item_maior_que_4_mb() {
        let mut h = History::default();
        let grande = vec![Flavor::new("text/html", vec![b'x'; MAX_ITEM_BYTES + 1])];
        assert_eq!(h.add(grande, 1), Added::TooLarge);
        assert!(h.items().is_empty());
    }

    #[test]
    fn fixado_sobrevive_ao_limite_e_ao_limpar() {
        let mut h = History::default();
        let Added::New(id) = h.add(txt("fixo"), 0) else {
            panic!()
        };
        h.toggle_pin(id);
        for i in 0..40 {
            h.add(txt(&format!("{i}")), i);
        }
        assert_eq!(h.items().len(), 25);
        assert!(h.get(id).is_some());
        h.clear();
        assert_eq!(h.items().len(), 1);
        assert_eq!(h.items()[0].id, id);
    }

    #[test]
    fn ids_nao_colidem_com_fixados_restaurados() {
        let clip = Clip {
            id: 7,
            flavors: txt("x"),
            pinned: true,
            copied_at: 0,
        };
        let mut h = History::with_pinned(vec![clip]);
        assert_eq!(h.add(txt("y"), 1), Added::New(8));
    }

    #[test]
    fn busca_ignora_acentos_e_caixa() {
        let mut h = History::default();
        h.add(txt("Reunião às três"), 1);
        h.add(txt("outra coisa"), 2);
        assert_eq!(h.search("REUNIAO").len(), 1);
        assert_eq!(h.search("").len(), 2);
    }

    #[test]
    fn ignora_conteudo_vazio() {
        let mut h = History::default();
        assert_eq!(h.add(txt(""), 1), Added::Empty);
    }
}
