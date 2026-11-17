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
