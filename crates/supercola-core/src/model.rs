//! Um item do histórico: o mesmo conteúdo em um ou mais formatos.
use std::sync::Arc;

pub const MIME_TEXT: &str = "text/plain;charset=utf-8";
pub const MIME_HTML: &str = "text/html";
pub const MIME_PNG: &str = "image/png";
pub const MIME_GIF: &str = "image/gif";
pub const MIME_URIS: &str = "text/uri-list";
/// Marca o que o próprio Super Cola colou e que não deve entrar no histórico.
pub const MIME_TRANSIENT: &str = "application/x-supercola-transient";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Flavor {
    pub mime: String,
    pub data: Arc<Vec<u8>>,
}

impl Flavor {
    pub fn new(mime: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            mime: mime.into(),
            data: Arc::new(data),
        }
    }
    pub fn text(text: &str) -> Self {
        Self::new(MIME_TEXT, text.as_bytes().to_vec())
    }
}

const TEXTOS: &[&str] = &[
    MIME_TEXT,
    "text/plain;charset=UTF-8",
    "UTF8_STRING",
    "text/plain",
    "STRING",
    "TEXT",
];

/// Escolhe o que ler de uma seleção: `(nome canônico, nome oferecido)`.
pub fn wanted(oferecidos: &[String]) -> Vec<(String, String)> {
    let achar = |m: &str| {
        oferecidos
            .iter()
            .find(|o| o.eq_ignore_ascii_case(m))
            .cloned()
    };
    let mut saida = Vec::new();
    if let Some(o) = TEXTOS.iter().find_map(|m| achar(m)) {
        saida.push((MIME_TEXT.to_string(), o));
    }
    for mime in [MIME_HTML, MIME_URIS, MIME_PNG] {
        if let Some(o) = achar(mime) {
            saida.push((mime.to_string(), o));
        }
    }
    saida
}

/// O que anunciar ao gravar, com os apelidos que apps antigos e XWayland pedem.
pub fn offered(flavors: &[Flavor], transient: bool) -> Vec<(String, Arc<Vec<u8>>)> {
    let mut saida = Vec::new();
    for f in flavors {
        saida.push((f.mime.clone(), f.data.clone()));
        if f.mime == MIME_TEXT {
            for apelido in ["text/plain", "UTF8_STRING", "STRING", "TEXT"] {
                saida.push((apelido.to_string(), f.data.clone()));
            }
        }
    }
    if transient {
        saida.push((MIME_TRANSIENT.to_string(), Arc::new(vec![1])));
    }
    saida
}

/// O GDK só serve tipos com barra. Apelidos como `UTF8_STRING` seriam anunciados e
/// chegariam vazios, e são os preferidos dos apps X11.
pub fn so_para_gdk(oferta: &[(String, Arc<Vec<u8>>)]) -> Vec<&(String, Arc<Vec<u8>>)> {
    oferta
        .iter()
        .filter(|(mime, _)| mime.contains('/'))
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    Image,
}

#[derive(Clone, Debug)]
pub struct Clip {
    pub id: u64,
    pub flavors: Vec<Flavor>,
    pub pinned: bool,
    pub copied_at: u64,
}

impl Clip {
    pub fn kind(&self) -> Kind {
        if self.flavor(MIME_PNG).is_some() && self.flavor(MIME_TEXT).is_none() {
            Kind::Image
        } else {
            Kind::Text
        }
    }

    pub fn flavor(&self, mime: &str) -> Option<&Flavor> {
        self.flavors.iter().find(|f| f.mime == mime)
    }

    pub fn text(&self) -> Option<&str> {
        std::str::from_utf8(&self.flavor(MIME_TEXT)?.data).ok()
    }

    /// Dois itens são o mesmo quando o conteúdo principal é igual, mesmo que
    /// um app ofereça também HTML e outro não.
    pub fn same_content(&self, flavors: &[Flavor]) -> bool {
        let main = |fs: &[Flavor]| {
            fs.iter()
                .find(|f| f.mime == MIME_TEXT)
                .or_else(|| fs.iter().find(|f| f.mime == MIME_PNG))
                .cloned()
        };
        main(&self.flavors).is_some() && main(&self.flavors) == main(flavors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gdk_recebe_so_tipos_que_consegue_servir() {
        let oferta = offered(&[Flavor::text("oi")], true);
        let nomes: Vec<&str> = so_para_gdk(&oferta)
            .iter()
            .map(|(mime, _)| mime.as_str())
            .collect();
        assert_eq!(nomes, [MIME_TEXT, "text/plain", MIME_TRANSIENT]);
    }

    fn mimes(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn prefere_utf8_e_guarda_html_e_imagem() {
        let r = wanted(&mimes(&[
            "STRING",
            "text/html",
            "text/plain;charset=utf-8",
            "image/png",
            "image/bmp",
        ]));
        assert_eq!(r[0], (MIME_TEXT.to_string(), MIME_TEXT.to_string()));
        assert_eq!(
            r.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(),
            [MIME_TEXT, MIME_HTML, MIME_PNG]
        );
    }

    #[test]
    fn texto_de_app_x11_vira_o_nome_canonico() {
        let r = wanted(&mimes(&["UTF8_STRING", "TARGETS"]));
        assert_eq!(r, [(MIME_TEXT.to_string(), "UTF8_STRING".to_string())]);
    }

    #[test]
    fn oferta_transitoria_leva_a_marca() {
        let o = offered(&[Flavor::text("x")], true);
        assert!(o.iter().any(|(m, _)| m == MIME_TRANSIENT));
        assert!(o.iter().any(|(m, _)| m == "UTF8_STRING"));
        assert!(!offered(&[Flavor::text("x")], false)
            .iter()
            .any(|(m, _)| m == MIME_TRANSIENT));
    }

    #[test]
    fn mesmo_texto_com_e_sem_html_e_o_mesmo_item() {
        let c = Clip {
            id: 1,
            flavors: vec![Flavor::text("a"), Flavor::new(MIME_HTML, vec![1])],
            pinned: false,
            copied_at: 0,
        };
        assert!(c.same_content(&[Flavor::text("a")]));
        assert!(!c.same_content(&[Flavor::text("b")]));
    }
}
