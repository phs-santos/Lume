//! Livro de exemplo para os testes da IA.
use crate::db::{books, scenes};
use rusqlite::Connection;

pub fn doc(paragraphs: &[&str]) -> String {
    let content: Vec<_> = paragraphs
        .iter()
        .map(|p| serde_json::json!({"type":"paragraph","content":[{"type":"text","text":p}]}))
        .collect();
    serde_json::json!({"type":"doc","content":content}).to_string()
}

pub struct Book {
    pub id: String,
    pub chapter: String,
    pub scenes: Vec<String>,
}

/// Livro com 1 capítulo e 3 cenas com texto.
pub fn sample(c: &Connection, mode: &str) -> Book {
    let b = books::book_create(c, "Até ter certeza", mode, 0).unwrap();
    let ch = books::book_outline(c, &b.id).unwrap().chapters[0].chapter.id.clone();
    let first = books::book_outline(c, &b.id).unwrap().chapters[0].scenes[0].id.clone();
    scenes::scene_rename(c, &first, "Linha 340").unwrap();
    scenes::scene_save(c, &first, &doc(&["A tela do notebook é a única luz do quarto.", "Rafa não dorme."]), 10, 1).unwrap();
    let s2 = scenes::scene_create(c, &ch, "A Porta do Elevador", None, 1).unwrap();
    scenes::scene_save(c, &s2.id, &doc(&["No elevador, Rafa descobre a verdade sobre o código."]), 9, 1).unwrap();
    let s3 = scenes::scene_create(c, &ch, "O Troco", None, 1).unwrap();
    scenes::scene_save(c, &s3.id, &doc(&["A padaria da esquina abre às seis."]), 7, 1).unwrap();
    Book { id: b.id, chapter: ch, scenes: vec![first, s2.id, s3.id] }
}

use crate::ai::provider::{ChatModel, Msg, ProviderError, ToolCall, ToolDef, Turn, Usage};
use std::sync::Mutex;

/// Modelo falso: devolve respostas roteirizadas e registra cada chamada.
pub struct FakeModel {
    pub script: Mutex<Vec<Result<Turn, ProviderError>>>,
    /// (modelo, nº de mensagens, nº de ferramentas oferecidas, último conteúdo)
    pub calls: Mutex<Vec<(String, usize, usize, String)>>,
    /// Quando o roteiro acaba: pede sempre uma ferramenta (se houver) ou responde "fim".
    pub always_tool: bool,
}

impl FakeModel {
    pub fn new(script: Vec<Result<Turn, ProviderError>>) -> Self {
        Self { script: Mutex::new(script.into_iter().rev().collect()), calls: Mutex::new(vec![]), always_tool: false }
    }
    pub fn n_calls(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

pub fn text_turn(t: &str) -> Result<Turn, ProviderError> {
    Ok(Turn { text: t.into(), tool_calls: vec![], usage: Usage { input_tokens: 10, output_tokens: 2, cost: None } })
}

pub fn tool_turn(name: &str, args: serde_json::Value) -> Result<Turn, ProviderError> {
    Ok(Turn {
        text: String::new(),
        tool_calls: vec![ToolCall { id: format!("c-{name}"), name: name.into(), arguments_json: args.to_string() }],
        usage: Usage { input_tokens: 10, output_tokens: 2, cost: None },
    })
}

impl ChatModel for FakeModel {
    fn complete(&self, model: &str, msgs: &[Msg], tools: &[ToolDef], on_delta: &mut dyn FnMut(&str)) -> Result<Turn, ProviderError> {
        self.calls.lock().unwrap().push((model.into(), msgs.len(), tools.len(), msgs.last().map(|m| m.content.clone()).unwrap_or_default()));
        let next = self.script.lock().unwrap().pop();
        let r = match next {
            Some(r) => r,
            None if self.always_tool && !tools.is_empty() => tool_turn("buscar_no_livro", serde_json::json!({"texto":"Rafa"})),
            None => text_turn("fim"),
        };
        if let Ok(t) = &r {
            for w in t.text.split_inclusive(' ') {
                on_delta(w);
            }
        }
        r
    }
}

pub fn shared(c: Connection) -> crate::state::SharedDb {
    std::sync::Arc::new(std::sync::Mutex::new(Some(c)))
}
