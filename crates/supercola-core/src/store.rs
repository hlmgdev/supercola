//! Só os itens fixados vão para o disco, como no Windows.
use std::fs;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

use rusqlite::{params, Connection};

use crate::model::{Clip, Flavor};

pub struct Store(Connection);

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(pai) = path.parent() {
            let _ = fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(pai);
        }
        // Cria com 0600 antes de o SQLite abrir, para nunca existir legível por outros.
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(path);
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
        let con = Connection::open(path)?;
        con.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS clip(
                id INTEGER PRIMARY KEY, pos INTEGER NOT NULL, copied_at INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS flavor(
                clip_id INTEGER NOT NULL REFERENCES clip(id) ON DELETE CASCADE,
                mime TEXT NOT NULL, data BLOB NOT NULL);",
        )?;
        Ok(Self(con))
    }

    pub fn load(&self) -> rusqlite::Result<Vec<Clip>> {
        let mut clips = self
            .0
            .prepare("SELECT id, copied_at FROM clip ORDER BY pos")?
            .query_map([], |l| {
                Ok(Clip {
                    id: l.get::<_, i64>(0)? as u64,
                    copied_at: l.get::<_, i64>(1)? as u64,
                    flavors: Vec::new(),
                    pinned: true,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut sabores = self
            .0
            .prepare("SELECT mime, data FROM flavor WHERE clip_id = ?1")?;
        for clip in &mut clips {
            clip.flavors = sabores
                .query_map([clip.id as i64], |l| {
                    Ok(Flavor::new(l.get::<_, String>(0)?, l.get(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
        }
        Ok(clips)
    }

    pub fn save(&mut self, pinned: &[Clip]) -> rusqlite::Result<()> {
        let tx = self.0.transaction()?;
        tx.execute("DELETE FROM flavor", [])?;
        tx.execute("DELETE FROM clip", [])?;
        for (pos, clip) in pinned.iter().enumerate() {
            tx.execute(
                "INSERT INTO clip(id, pos, copied_at) VALUES (?1, ?2, ?3)",
                params![clip.id as i64, pos as i64, clip.copied_at as i64],
            )?;
            for f in &clip.flavors {
                tx.execute(
                    "INSERT INTO flavor(clip_id, mime, data) VALUES (?1, ?2, ?3)",
                    params![clip.id as i64, f.mime, f.data.as_slice()],
                )?;
            }
        }
        tx.commit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MIME_PNG;

    #[test]
    fn fixados_voltam_iguais_e_o_arquivo_e_privado() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/pinned.db");
        let clips = vec![
            Clip {
                id: 3,
                flavors: vec![Flavor::text("olá")],
                pinned: true,
                copied_at: 9,
            },
            Clip {
                id: 1,
                flavors: vec![Flavor::new(MIME_PNG, vec![1, 0, 2])],
                pinned: true,
                copied_at: 4,
            },
        ];
        Store::open(&path).unwrap().save(&clips).unwrap();
        let lidos = Store::open(&path).unwrap().load().unwrap();
        assert_eq!(lidos[0].id, 3);
        assert_eq!(lidos[0].text(), Some("olá"));
        assert_eq!(*lidos[1].flavors[0].data, vec![1, 0, 2]);
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn salvar_substitui_o_que_havia() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("p.db")).unwrap();
        let c = Clip {
            id: 1,
            flavors: vec![Flavor::text("a")],
            pinned: true,
            copied_at: 0,
        };
        s.save(&[c]).unwrap();
        s.save(&[]).unwrap();
        assert!(s.load().unwrap().is_empty());
    }
}
