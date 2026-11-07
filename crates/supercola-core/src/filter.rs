//! O que nunca entra no histórico, e quais apps colam diferente.

/// Formatos com que gerenciadores de senha marcam conteúdo secreto.
const SECRET_MIMES: &[&str] = &[
    "x-kde-passwordManagerHint",
    "org.nspasteboard.ConcealedType",
    "application/x-nspasteboard-concealed-type",
];

pub fn is_secret(mimes: &[String]) -> bool {
    mimes
        .iter()
        .any(|m| SECRET_MIMES.iter().any(|s| m.eq_ignore_ascii_case(s)))
}

pub fn in_list(app_id: Option<&str>, lista: &[String]) -> bool {
    app_id.is_some_and(|a| lista.iter().any(|e| e.eq_ignore_ascii_case(a)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_marca_de_senha() {
        assert!(is_secret(&[
            "text/plain".into(),
            "x-kde-passwordManagerHint".into()
        ]));
        assert!(!is_secret(&["text/plain".into()]));
    }

    #[test]
    fn lista_nao_diferencia_caixa() {
        let lista = vec!["org.keepassxc.KeePassXC".to_string()];
        assert!(in_list(Some("org.KeePassXC.keepassxc"), &lista));
        assert!(!in_list(Some("firefox"), &lista));
        assert!(!in_list(None, &lista));
    }
}
