//! Preferências em `~/.config/supercola/config.toml`.
use std::fs;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    #[default]
    BottomRight,
    NearPointer,
}

/// Como gravar no clipboard.
///
/// `DataControl` serve cada leitor em uma thread própria, sem travar a interface, e é o
/// mesmo caminho que devolve o clipboard depois de um emoji. `Gdk` é uma cópia comum,
/// feita com o painel em foco.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WriteBackend {
    #[default]
    DataControl,
    Gdk,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    #[default]
    Auto,
    Pt,
    En,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GifProvider {
    #[default]
    Klipy,
    Giphy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub history_enabled: bool,
    pub auto_paste: bool,
    pub paste_delay_ms: u64,
    pub position: Position,
    pub write_backend: WriteBackend,
    pub theme: ThemeMode,
    pub language: Language,
    pub terminals: Vec<String>,
    pub excluded_apps: Vec<String>,
    pub gif_provider: GifProvider,
    /// Informada pelo usuário; nunca vem embutida no pacote.
    pub gif_api_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            history_enabled: true,
            auto_paste: true,
            paste_delay_ms: 150,
            position: Position::default(),
            write_backend: WriteBackend::default(),
            theme: ThemeMode::default(),
            language: Language::default(),
            terminals: [
                "com.system76.CosmicTerm",
                "org.gnome.Terminal",
                "org.gnome.Console",
                "org.gnome.Ptyxis",
                "org.kde.konsole",
                "Alacritty",
                "kitty",
                "foot",
                "org.wezfurlong.wezterm",
                "com.gexperts.Tilix",
                "terminator",
                "xterm",
            ]
            .map(String::from)
            .into(),
            excluded_apps: [
                "org.keepassxc.KeePassXC",
                "com.bitwarden.desktop",
                "1password",
            ]
            .map(String::from)
            .into(),
            gif_provider: GifProvider::default(),
            gif_api_key: String::new(),
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        crate::xdg::config_dir().join("config.toml")
    }

    /// Arquivo ausente ou inválido cai nos padrões; o segundo valor conta o que houve.
    pub fn load() -> (Self, Option<String>) {
        match fs::read_to_string(Self::path()) {
            Ok(texto) => match toml::from_str(&texto) {
                Ok(c) => (c, None),
                Err(e) => (Self::default(), Some(e.to_string())),
            },
            Err(_) => (Self::default(), None),
        }
    }

    /// Pode conter a chave da API de GIF: o arquivo fica só para o dono.
    pub fn save(&self) -> std::io::Result<()> {
        use std::io::Write;
        let path = Self::path();
        if let Some(pai) = path.parent() {
            fs::create_dir_all(pai)?;
        }
        let texto = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("toml.tmp");
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        f.write_all(texto.as_bytes())?;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
        fs::rename(tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campo_ausente_usa_o_padrao() {
        let c: Config = toml::from_str("auto_paste = false").unwrap();
        assert!(!c.auto_paste);
        assert!(c.history_enabled);
        assert!(c.terminals.iter().any(|t| t == "com.system76.CosmicTerm"));
    }

    #[test]
    fn ida_e_volta() {
        let c = Config {
            position: Position::NearPointer,
            ..Config::default()
        };
        let t = toml::to_string_pretty(&c).unwrap();
        assert!(t.contains("near-pointer"));
        assert_eq!(toml::from_str::<Config>(&t).unwrap(), c);
    }
}
