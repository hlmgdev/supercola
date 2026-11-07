//! Comandos do `supercola-toggle` para o daemon, uma linha por conexão.
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Home,
    History,
    Emoji,
    Gif,
    Kaomoji,
    Symbols,
}

impl Tab {
    pub const ALL: [Tab; 6] = [
        Tab::Home,
        Tab::Emoji,
        Tab::Gif,
        Tab::Kaomoji,
        Tab::Symbols,
        Tab::History,
    ];

    pub fn parse(nome: &str) -> Option<Self> {
        Some(match nome {
            "home" => Self::Home,
            "history" => Self::History,
            "emoji" => Self::Emoji,
            "gif" => Self::Gif,
            "kaomoji" => Self::Kaomoji,
            "symbols" => Self::Symbols,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::History => "history",
            Self::Emoji => "emoji",
            Self::Gif => "gif",
            Self::Kaomoji => "kaomoji",
            Self::Symbols => "symbols",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Toggle(Tab),
    Hide,
    Settings,
    Status,
    Quit,
}

impl Command {
    pub fn parse(linha: &str) -> Option<Self> {
        let mut partes = linha.split_whitespace();
        Some(match partes.next()? {
            "toggle" => Self::Toggle(partes.next().map_or(Some(Tab::History), Tab::parse)?),
            "hide" => Self::Hide,
            "settings" => Self::Settings,
            "status" => Self::Status,
            "quit" => Self::Quit,
            _ => return None,
        })
    }
}

pub fn socket_path() -> PathBuf {
    crate::xdg::runtime_dir().join("supercola.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpreta_comandos() {
        assert_eq!(
            Command::parse("toggle"),
            Some(Command::Toggle(Tab::History))
        );
        assert_eq!(
            Command::parse("toggle emoji\n"),
            Some(Command::Toggle(Tab::Emoji))
        );
        assert_eq!(
            Command::parse("toggle home"),
            Some(Command::Toggle(Tab::Home))
        );
        assert_eq!(Command::parse("toggle xyz"), None);
        assert_eq!(Command::parse("rm -rf"), None);
        assert_eq!(Command::parse(""), None);
    }
}
