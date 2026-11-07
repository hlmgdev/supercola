//! Conteúdo determinístico: o alvo compara o hash, sem depender de olho humano.

const LINE: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ+\n";

pub fn make(size: usize) -> String {
    if size == 0 {
        return "supercola-spike-ok".to_string();
    }
    LINE.repeat(size / LINE.len() + 1)[..size].to_string()
}

/// FNV-1a: basta para detectar corrupção, não é uso criptográfico.
pub fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
