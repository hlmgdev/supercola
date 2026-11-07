//! Registra o atalho no arquivo `custom` do COSMIC sem estragar o que já existe.
//!
//! O arquivo é RON: um mapa `{ (modifiers: [Super], key: "v"): Spawn("cmd"), }`.
//! Nada é reescrito além da inserção de uma entrada.

#[derive(Debug, PartialEq, Eq)]
pub enum Merge {
    /// O atalho já chama o nosso comando.
    AlreadySet,
    /// Texto novo para gravar.
    Updated(String),
    /// A tecla já faz outra coisa; não mexemos.
    Conflict(String),
    /// O arquivo não tem a forma esperada; não mexemos.
    Invalid,
}

/// Tira espaços fora de strings, para comparar independente da formatação.
fn compactar(texto: &str) -> String {
    let mut saida = String::with_capacity(texto.len());
    let mut em_string = false;
    let mut escape = false;
    for c in texto.chars() {
        if em_string {
            saida.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                em_string = false;
            }
        } else if c == '"' {
            em_string = true;
            saida.push(c);
        } else if !c.is_whitespace() {
            saida.push(c);
        }
    }
    saida
}

/// Posição do `}` que fecha o mapa, ignorando o que está dentro de strings.
fn fecho_do_mapa(texto: &str) -> Option<usize> {
    let mut em_string = false;
    let mut escape = false;
    let mut nivel = 0i32;
    let mut fim = None;
    for (i, c) in texto.char_indices() {
        if em_string {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                em_string = false;
            }
            continue;
        }
        match c {
            '"' => em_string = true,
            '{' => nivel += 1,
            '}' => {
                nivel -= 1;
                if nivel == 0 {
                    fim = Some(i);
                }
                if nivel < 0 {
                    return None;
                }
            }
            _ => {}
        }
    }
    if nivel == 0 && !em_string {
        fim
    } else {
        None
    }
}

pub fn merge(existente: Option<&str>, modifiers: &str, key: &str, command: &str) -> Merge {
    let entrada = format!("(modifiers: [{modifiers}], key: \"{key}\"): Spawn(\"{command}\")");
    let Some(texto) = existente.filter(|t| !t.trim().is_empty()) else {
        return Merge::Updated(format!("{{\n    {entrada},\n}}\n"));
    };
    let Some(fecho) = fecho_do_mapa(texto) else {
        return Merge::Invalid;
    };
    if !texto.trim_start().starts_with('{') {
        return Merge::Invalid;
    }

    let compacto = compactar(texto);
    let chave = format!("(modifiers:[{}],key:\"{key}\"", compactar(modifiers));
    if let Some(i) = compacto.find(&chave) {
        let resto = &compacto[i..];
        let Some(dois_pontos) = resto.find("):") else {
            return Merge::Invalid;
        };
        let acao = &resto[dois_pontos + 2..];
        return if acao.starts_with(&format!("Spawn(\"{command}\")")) {
            Merge::AlreadySet
        } else {
            let fim = acao
                .find(",(")
                .or_else(|| acao.rfind('}'))
                .unwrap_or(acao.len());
            Merge::Conflict(acao[..fim].trim_end_matches(',').to_string())
        };
    }

    let antes = texto[..fecho].trim_end();
    let virgula = if antes.ends_with('{') || antes.ends_with(',') {
        ""
    } else {
        ","
    };
    Merge::Updated(format!(
        "{antes}{virgula}\n    {entrada},\n{}",
        &texto[fecho..]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CMD: &str = "supercola-toggle";

    fn feito(m: Merge) -> String {
        match m {
            Merge::Updated(t) => t,
            outro => panic!("{outro:?}"),
        }
    }

    #[test]
    fn cria_arquivo_novo() {
        assert_eq!(
            feito(merge(None, "Super", "v", CMD)),
            "{\n    (modifiers: [Super], key: \"v\"): Spawn(\"supercola-toggle\"),\n}\n"
        );
        assert!(matches!(
            merge(Some("  \n"), "Super", "v", CMD),
            Merge::Updated(_)
        ));
    }

    #[test]
    fn preserva_atalhos_existentes() {
        let velho = "{\n    (modifiers: [Super], key: \"t\"): Spawn(\"foot\")\n}\n";
        let novo = feito(merge(Some(velho), "Super", "v", CMD));
        assert!(novo.contains("Spawn(\"foot\"),"));
        assert!(novo.contains("key: \"v\""));
        assert!(fecho_do_mapa(&novo).is_some());
    }

    #[test]
    fn nao_duplica() {
        let uma = feito(merge(None, "Super", "v", CMD));
        assert_eq!(merge(Some(&uma), "Super", "v", CMD), Merge::AlreadySet);
    }

    #[test]
    fn reconhece_mesmo_com_outra_formatacao() {
        let t = "{(modifiers:[ Super ],\n key:\"v\"):Spawn(\"supercola-toggle\")}";
        assert_eq!(merge(Some(t), "Super", "v", CMD), Merge::AlreadySet);
    }

    #[test]
    fn nao_sobrescreve_atalho_alheio() {
        let t = "{\n (modifiers: [Super], key: \"v\"): Spawn(\"outro\"),\n (modifiers: [Super], key: \"t\"): Close,\n}";
        assert_eq!(
            merge(Some(t), "Super", "v", CMD),
            Merge::Conflict("Spawn(\"outro\")".into())
        );
    }

    #[test]
    fn modificador_diferente_nao_e_conflito() {
        let t = "{ (modifiers: [Super, Shift], key: \"v\"): Close, }";
        assert!(matches!(
            merge(Some(t), "Super", "v", CMD),
            Merge::Updated(_)
        ));
    }

    #[test]
    fn chave_dentro_de_string_nao_confunde() {
        let t = "{ (modifiers: [Super], key: \"t\"): Spawn(\"echo } {\"), }";
        let novo = feito(merge(Some(t), "Super", "v", CMD));
        assert!(novo.starts_with("{ (modifiers: [Super], key: \"t\"): Spawn(\"echo } {\"),"));
    }

    #[test]
    fn recusa_arquivo_estranho() {
        assert_eq!(merge(Some("{ quebrado"), "Super", "v", CMD), Merge::Invalid);
        assert_eq!(merge(Some("(lista)"), "Super", "v", CMD), Merge::Invalid);
        assert_eq!(merge(Some("x { }"), "Super", "v", CMD), Merge::Invalid);
    }
}
