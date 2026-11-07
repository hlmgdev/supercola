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

/// O que fazer com uma seleção nova, só pelos formatos anunciados.
#[derive(Debug, PartialEq, Eq)]
pub enum Decisao {
    /// Escrita do próprio Super Cola que não deve voltar ao histórico.
    Ignorar,
    /// Conteúdo marcado como senha: não pode ser lido.
    Segredo,
    /// Formatos a ler: `(nome canônico, nome oferecido)`.
    Ler(Vec<(String, String)>),
}

pub fn decidir(mimes: &[String]) -> Decisao {
    if mimes.iter().any(|m| m == crate::model::MIME_TRANSIENT) {
        Decisao::Ignorar
    } else if is_secret(mimes) {
        Decisao::Segredo
    } else {
        Decisao::Ler(crate::model::wanted(mimes))
    }
}

pub fn in_list(app_id: Option<&str>, lista: &[String]) -> bool {
    app_id.is_some_and(|a| lista.iter().any(|e| e.eq_ignore_ascii_case(a)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mimes(nomes: &[&str]) -> Vec<String> {
        nomes.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn segredo_nunca_vira_leitura() {
        let m = mimes(&["text/plain", "x-kde-passwordManagerHint"]);
        assert_eq!(decidir(&m), Decisao::Segredo);
    }

    #[test]
    fn escrita_propria_vale_mais_que_segredo() {
        let m = mimes(&[crate::model::MIME_TRANSIENT, "x-kde-passwordManagerHint"]);
        assert_eq!(decidir(&m), Decisao::Ignorar);
    }

    #[test]
    fn selecao_comum_lista_o_que_ler() {
        let esperado = vec![(
            crate::model::MIME_TEXT.to_string(),
            "text/plain".to_string(),
        )];
        assert_eq!(decidir(&mimes(&["text/plain"])), Decisao::Ler(esperado));
    }

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
