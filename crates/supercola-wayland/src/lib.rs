//! Protocolos Wayland do Super Cola, cada um em conexão própria, fora da thread do GTK.
pub mod clipboard;
pub mod focus;
pub mod keyboard;
pub mod probe;

pub type R<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
