//! Texto plano de uma cena (ProseMirror JSON) e um hash estável dele.

use serde_json::Value;

fn inline_text(v: &Value, out: &mut String) {
    if let Some(t) = v.get("text").and_then(Value::as_str) {
        out.push_str(t);
    }
    if v.get("type").and_then(Value::as_str) == Some("hardBreak") {
        out.push('\n');
    }
    if let Some(children) = v.get("content").and_then(Value::as_array) {
        for ch in children {
            inline_text(ch, out);
        }
    }
}

/// Blocos de primeiro nível separados por linha em branco. JSON inválido vira texto vazio.
pub fn plain_text(content_json: &str) -> String {
    let Ok(doc) = serde_json::from_str::<Value>(content_json) else { return String::new() };
    let blocks = doc.get("content").and_then(Value::as_array).cloned().unwrap_or_default();
    blocks
        .iter()
        .map(|b| {
            let mut s = String::new();
            inline_text(b, &mut s);
            s
        })
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// FNV-1a 64 do texto plano: estável entre versões do Rust e indiferente à formatação.
pub fn content_hash(content_json: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in plain_text(content_json).as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    const DOC: &str = r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"A "},{"type":"text","marks":[{"type":"italic"}],"text":"b"}]},{"type":"paragraph","content":[{"type":"text","text":"C"}]}]}"#;

    #[test]
    fn plain_text_joins_paragraphs() {
        assert_eq!(plain_text(DOC), "A b\n\nC");
        assert_eq!(plain_text("não é json"), "");
    }

    #[test]
    fn hash_ignores_formatting() {
        let plain = r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"A b"}]},{"type":"paragraph","content":[{"type":"text","text":"C"}]}]}"#;
        assert_eq!(content_hash(DOC), content_hash(plain));
        assert_ne!(content_hash(DOC), content_hash(r#"{"type":"doc","content":[]}"#));
    }
}
