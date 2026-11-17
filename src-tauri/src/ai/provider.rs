//! Provedor de IA compatível com OpenAI (Inworld Router), atrás de um trait para os testes.

use serde_json::{json, Value};
use std::io::BufRead;

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments_json: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Msg {
    /// system | user | assistant | tool
    pub role: String,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub tool_call_id: Option<String>,
}

impl Msg {
    fn new(role: &str, content: &str) -> Self {
        Self { role: role.into(), content: content.into(), tool_calls: vec![], tool_call_id: None }
    }
    pub fn system(c: &str) -> Self { Self::new("system", c) }
    pub fn user(c: &str) -> Self { Self::new("user", c) }
    pub fn assistant(c: &str) -> Self { Self::new("assistant", c) }
    pub fn assistant_tools(c: &str, calls: Vec<ToolCall>) -> Self {
        Self { tool_calls: calls, ..Self::new("assistant", c) }
    }
    pub fn tool(call_id: &str, c: &str) -> Self {
        Self { tool_call_id: Some(call_id.into()), ..Self::new("tool", c) }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Usage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Turn {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Chave da Inworld inválida")]
    Auth,
    #[error("Sem crédito ou limite atingido na Inworld")]
    Quota,
    #[error("Sem conexão com a Inworld: {0}")]
    Network(String),
    #[error("A IA respondeu com erro: {0}")]
    Bad(String),
}

pub trait ChatModel: Send + Sync {
    /// Uma chamada ao modelo. `on_delta` recebe o texto à medida que chega.
    fn complete(&self, model: &str, msgs: &[Msg], tools: &[ToolDef], on_delta: &mut dyn FnMut(&str)) -> Result<Turn, ProviderError>;
}

pub fn msg_json(m: &Msg) -> Value {
    let mut v = json!({ "role": m.role, "content": m.content });
    if !m.tool_calls.is_empty() {
        v["tool_calls"] = m
            .tool_calls
            .iter()
            .map(|t| json!({ "id": t.id, "type": "function", "function": { "name": t.name, "arguments": t.arguments_json } }))
            .collect();
    }
    if let Some(id) = &m.tool_call_id {
        v["tool_call_id"] = json!(id);
    }
    v
}

pub fn status_error(status: u16, body: &str) -> ProviderError {
    match status {
        401 | 403 => ProviderError::Auth,
        402 | 429 => ProviderError::Quota,
        _ => ProviderError::Bad(format!("{status} {}", body.chars().take(200).collect::<String>())),
    }
}

/// Lê um stream SSE no formato OpenAI e junta texto, chamadas de ferramenta e uso.
pub fn parse_sse(body: impl BufRead, on_delta: &mut dyn FnMut(&str)) -> Result<Turn, ProviderError> {
    let mut turn = Turn::default();
    let mut calls: Vec<ToolCall> = vec![];
    for line in body.lines() {
        let line = line.map_err(|e| ProviderError::Network(e.to_string()))?;
        let Some(data) = line.strip_prefix("data:") else { continue };
        let data = data.trim();
        if data == "[DONE]" {
            break;
        }
        let Ok(v) = serde_json::from_str::<Value>(data) else { continue };
        if let Some(err) = v.get("error") {
            return Err(ProviderError::Bad(err.to_string()));
        }
        if let Some(u) = v.get("usage").filter(|u| !u.is_null()) {
            turn.usage.input_tokens = u["prompt_tokens"].as_i64().unwrap_or(0);
            turn.usage.output_tokens = u["completion_tokens"].as_i64().unwrap_or(0);
            turn.usage.cost = u.get("cost").and_then(Value::as_f64);
        }
        let Some(delta) = v.pointer("/choices/0/delta") else { continue };
        if let Some(t) = delta.get("content").and_then(Value::as_str) {
            if !t.is_empty() {
                turn.text.push_str(t);
                on_delta(t);
            }
        }
        for tc in delta.get("tool_calls").and_then(Value::as_array).into_iter().flatten() {
            let i = tc["index"].as_u64().unwrap_or(0) as usize;
            while calls.len() <= i {
                calls.push(ToolCall { id: String::new(), name: String::new(), arguments_json: String::new() });
            }
            let c = &mut calls[i];
            // pedaços seguintes vêm com "id":"" — não podem apagar o id do primeiro
            if let Some(id) = tc.get("id").and_then(Value::as_str).filter(|id| !id.is_empty()) { c.id = id.into(); }
            if let Some(n) = tc.pointer("/function/name").and_then(Value::as_str) { c.name.push_str(n); }
            if let Some(a) = tc.pointer("/function/arguments").and_then(Value::as_str) { c.arguments_json.push_str(a); }
        }
    }
    turn.tool_calls = calls
        .into_iter()
        .enumerate()
        .filter(|(_, c)| !c.name.is_empty())
        .map(|(i, mut c)| {
            if c.id.is_empty() {
                c.id = format!("call_{i}"); // a resposta da ferramenta precisa de um id
            }
            c
        })
        .collect();
    Ok(turn)
}

/// Inworld Router (OpenAI-compatível). A chave só existe aqui, no Rust.
pub struct Inworld {
    pub key: String,
}

const INWORLD_URL: &str = "https://api.inworld.ai/v1/chat/completions";

impl ChatModel for Inworld {
    fn complete(&self, model: &str, msgs: &[Msg], tools: &[ToolDef], on_delta: &mut dyn FnMut(&str)) -> Result<Turn, ProviderError> {
        let mut body = json!({
            "model": model,
            "stream": true,
            "stream_options": { "include_usage": true },
            "messages": msgs.iter().map(msg_json).collect::<Vec<_>>(),
        });
        if !tools.is_empty() {
            body["tools"] = tools
                .iter()
                .map(|t| json!({ "type": "function", "function": { "name": t.name, "description": t.description, "parameters": t.parameters } }))
                .collect();
        }
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| ProviderError::Network(e.to_string()))?;
        let resp = client
            .post(INWORLD_URL)
            .header("Authorization", format!("Basic {}", self.key.trim()))
            .json(&body)
            .send()
            .map_err(|e| ProviderError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(status_error(status, &resp.text().unwrap_or_default()));
        }
        parse_sse(std::io::BufReader::new(resp), on_delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sse(lines: &[&str]) -> std::io::Cursor<Vec<u8>> {
        std::io::Cursor::new(lines.iter().map(|l| format!("data: {l}\n\n")).collect::<String>().into_bytes())
    }

    #[test]
    fn parse_sse_text_and_usage() {
        let body = sse(&[
            r#"{"choices":[{"index":0,"delta":{"role":"assistant","content":"Ol"}}]}"#,
            r#"{"choices":[{"index":0,"delta":{"content":"á, "}}]}"#,
            r#"{"choices":[{"index":0,"delta":{"content":"tudo"}}]}"#,
            r#"{"choices":[],"usage":{"prompt_tokens":12,"completion_tokens":3}}"#,
            "[DONE]",
        ]);
        let mut deltas = vec![];
        let t = parse_sse(body, &mut |d| deltas.push(d.to_string())).unwrap();
        assert_eq!(t.text, "Olá, tudo");
        assert_eq!(deltas.len(), 3);
        assert_eq!((t.usage.input_tokens, t.usage.output_tokens), (12, 3));
        assert!(t.tool_calls.is_empty());
    }

    #[test]
    fn parse_sse_tool_call_split_across_chunks() {
        let body = sse(&[
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"c1","type":"function","function":{"name":"ler_cena","arguments":""}}]}}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"id\":"}}]}}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"s1\"}"}}]},"finish_reason":"tool_calls"}]}"#,
            "[DONE]",
        ]);
        let t = parse_sse(body, &mut |_| {}).unwrap();
        assert_eq!(t.tool_calls, vec![ToolCall { id: "c1".into(), name: "ler_cena".into(), arguments_json: r#"{"id":"s1"}"#.into() }]);
    }

    #[test]
    fn parse_sse_parallel_calls_keep_ids_when_later_chunks_send_empty_id() {
        // formato real da Inworld: pedaços seguintes trazem "id":"" e "name":""
        let body = sse(&[
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"toolu_A","type":"function","function":{"name":"ler_cena","arguments":""}}]}}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"","type":"function","function":{"name":"","arguments":"{\"id\":\"a1\"}"}}]}}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"id":"toolu_B","type":"function","function":{"name":"ler_cena","arguments":""}}]}}]}"#,
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"id":"","type":"function","function":{"name":"","arguments":"{\"id\":\"a2\"}"}}]}}]}"#,
            "[DONE]",
        ]);
        let t = parse_sse(body, &mut |_| {}).unwrap();
        assert_eq!(t.tool_calls.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), vec!["toolu_A", "toolu_B"]);
        assert_eq!(t.tool_calls[1].arguments_json, r#"{"id":"a2"}"#);
    }

    #[test]
    fn parse_sse_fills_missing_call_id() {
        let body = sse(&[r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"name":"ler_cena","arguments":"{}"}}]}}]}"#, "[DONE]"]);
        let t = parse_sse(body, &mut |_| {}).unwrap();
        assert!(!t.tool_calls[0].id.is_empty());
    }

    #[test]
    fn http_status_maps_to_error() {
        assert!(matches!(status_error(401, ""), ProviderError::Auth));
        assert!(matches!(status_error(402, ""), ProviderError::Quota));
        assert!(matches!(status_error(429, ""), ProviderError::Quota));
        assert!(matches!(status_error(500, "x"), ProviderError::Bad(_)));
    }

    #[test]
    fn messages_serialize_openai_shape() {
        let m = Msg::assistant_tools("", vec![ToolCall { id: "c1".into(), name: "n".into(), arguments_json: "{}".into() }]);
        let v = msg_json(&m);
        assert_eq!(v["tool_calls"][0]["function"]["name"], "n");
        let t = msg_json(&Msg::tool("c1", "ok"));
        assert_eq!(t["tool_call_id"], "c1");
    }

    #[test]
    #[ignore]
    fn live_inworld_says_ok() {
        let key = std::env::var("INWORLD_KEY").expect("INWORLD_KEY");
        let t = Inworld { key }.complete("anthropic/claude-haiku-5-5", &[Msg::user("Responda só: ok")], &[], &mut |_| {}).unwrap();
        assert!(t.text.to_lowercase().contains("ok"));
        assert!(t.usage.input_tokens > 0);
    }
}
