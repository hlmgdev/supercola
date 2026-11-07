//! Provedores de GIF: monta a URL e interpreta a resposta, sem tocar na rede.
//!
//! A KLIPY é lida de forma tolerante: a documentação pública varia entre versões,
//! então aceitamos a lista em `data.data`, `data` ou `results`.
use serde_json::Value;

use crate::config::GifProvider;

const LIMITE: u32 = 24;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GifItem {
    pub id: String,
    pub title: String,
    pub preview_url: String,
    pub full_url: String,
}

fn codificar(texto: &str) -> String {
    let mut saida = String::new();
    for b in texto.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                saida.push(b as char)
            }
            _ => saida.push_str(&format!("%{b:02X}")),
        }
    }
    saida
}

/// Sem `query`, pede os mais populares.
pub fn request_url(
    provedor: GifProvider,
    chave: &str,
    query: Option<&str>,
    locale: &str,
) -> String {
    let chave = codificar(chave);
    let locale = codificar(locale);
    let query = query
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(codificar);
    match (provedor, query) {
        (GifProvider::Klipy, Some(q)) => format!(
            "https://api.klipy.com/api/v1/{chave}/gifs/search?q={q}&per_page={LIMITE}&page=1&customer_id=supercola&locale={locale}"
        ),
        (GifProvider::Klipy, None) => format!(
            "https://api.klipy.com/api/v1/{chave}/gifs/trending?per_page={LIMITE}&page=1&customer_id=supercola&locale={locale}"
        ),
        (GifProvider::Giphy, Some(q)) => format!(
            "https://api.giphy.com/v1/gifs/search?api_key={chave}&q={q}&limit={LIMITE}&rating=g&lang={locale}"
        ),
        (GifProvider::Giphy, None) => format!(
            "https://api.giphy.com/v1/gifs/trending?api_key={chave}&limit={LIMITE}&rating=g"
        ),
    }
}

fn texto(v: &Value, chave: &str) -> String {
    match v.get(chave) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn so_https(url: String) -> Option<String> {
    url.starts_with("https://").then_some(url)
}

fn klipy_url(item: &Value, tamanhos: &[&str]) -> Option<String> {
    let arquivos = item.get("file")?;
    tamanhos.iter().find_map(|t| {
        let tam = arquivos.get(t)?;
        let url = tam
            .get("gif")
            .and_then(|f| f.get("url"))
            .and_then(Value::as_str)?;
        so_https(url.to_string())
    })
}

fn klipy(raiz: &Value) -> Option<Vec<GifItem>> {
    let lista = raiz
        .pointer("/data/data")
        .or_else(|| raiz.get("data"))
        .or_else(|| raiz.get("results"))?
        .as_array()?;
    Some(
        lista
            .iter()
            .filter_map(|i| {
                Some(GifItem {
                    id: texto(i, "id"),
                    title: texto(i, "title"),
                    preview_url: klipy_url(i, &["sm", "xs", "md", "hd"])?,
                    full_url: klipy_url(i, &["md", "hd", "sm", "xs"])?,
                })
            })
            .collect(),
    )
}

fn giphy(raiz: &Value) -> Option<Vec<GifItem>> {
    let url = |i: &Value, nome: &str| {
        i.pointer(&format!("/images/{nome}/url"))
            .and_then(Value::as_str)
            .and_then(|u| so_https(u.to_string()))
    };
    Some(
        raiz.get("data")?
            .as_array()?
            .iter()
            .filter_map(|i| {
                Some(GifItem {
                    id: texto(i, "id"),
                    title: texto(i, "title"),
                    preview_url: url(i, "fixed_width_small").or_else(|| url(i, "fixed_width"))?,
                    full_url: url(i, "downsized").or_else(|| url(i, "original"))?,
                })
            })
            .collect(),
    )
}

pub fn parse(provedor: GifProvider, corpo: &[u8]) -> Result<Vec<GifItem>, String> {
    let raiz: Value = serde_json::from_slice(corpo).map_err(|e| e.to_string())?;
    match provedor {
        GifProvider::Klipy => klipy(&raiz),
        GifProvider::Giphy => giphy(&raiz),
    }
    .ok_or_else(|| "resposta em formato inesperado".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codifica_a_busca() {
        let u = request_url(GifProvider::Giphy, "k", Some("café com leite"), "pt");
        assert!(u.contains("q=caf%C3%A9%20com%20leite"), "{u}");
    }

    #[test]
    fn busca_vazia_vira_populares() {
        let u = request_url(GifProvider::Klipy, "k", Some("  "), "pt_BR");
        assert!(u.contains("/gifs/trending?"), "{u}");
    }

    #[test]
    fn le_klipy_aninhado() {
        let corpo = br#"{"result":true,"data":{"data":[{"id":7,"title":"oi","file":{
            "hd":{"gif":{"url":"https://x/hd.gif"}},
            "md":{"gif":{"url":"https://x/md.gif"}},
            "sm":{"gif":{"url":"https://x/sm.gif"}}}}],"has_next":true}}"#;
        let r = parse(GifProvider::Klipy, corpo).unwrap();
        assert_eq!(r[0].id, "7");
        assert_eq!(r[0].preview_url, "https://x/sm.gif");
        assert_eq!(r[0].full_url, "https://x/md.gif");
    }

    #[test]
    fn le_giphy() {
        let corpo = br#"{"data":[{"id":"a","title":"t","images":{
            "fixed_width_small":{"url":"https://g/s.gif"},
            "downsized":{"url":"https://g/d.gif"}}}]}"#;
        let r = parse(GifProvider::Giphy, corpo).unwrap();
        assert_eq!(r[0].full_url, "https://g/d.gif");
    }

    #[test]
    fn recusa_url_sem_https_e_json_estranho() {
        let corpo = br#"{"data":[{"id":"a","images":{"original":{"url":"http://g/o.gif"}}}]}"#;
        assert!(parse(GifProvider::Giphy, corpo).unwrap().is_empty());
        assert!(parse(GifProvider::Klipy, b"{}").is_err());
        assert!(parse(GifProvider::Klipy, b"xx").is_err());
    }
}
