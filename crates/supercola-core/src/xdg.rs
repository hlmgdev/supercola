//! Diretórios por usuário.
use std::path::PathBuf;

fn base(var: &str, padrao: &str) -> PathBuf {
    match std::env::var_os(var) {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(padrao),
    }
}

pub fn config_home() -> PathBuf {
    base("XDG_CONFIG_HOME", ".config")
}
pub fn config_dir() -> PathBuf {
    config_home().join("supercola")
}
pub fn data_dir() -> PathBuf {
    base("XDG_DATA_HOME", ".local/share").join("supercola")
}
pub fn cache_dir() -> PathBuf {
    base("XDG_CACHE_HOME", ".cache").join("supercola")
}
pub fn runtime_dir() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => std::env::temp_dir(),
    }
}
