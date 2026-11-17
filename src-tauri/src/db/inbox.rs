//! Ideias recebidas do celular, já decifradas (dentro do banco cifrado).

use super::DbError;
use crate::db::scenes::{self, Scene};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct InboxItem {
    pub id: String,
    pub kind: String,
    pub mime: String,
    pub size: i64,
    pub created_at: i64,
    pub status: String,
    /// Texto da ideia (só para `kind = text`); áudio vem por `inbox_body`.
    pub text: Option<String>,
}

use rusqlite::{params, OptionalExtension};

pub fn inbox_has(c: &Connection, id: &str) -> Result<bool, DbError> {
    Ok(c.query_row("SELECT 1 FROM inbox WHERE id=?1", [id], |_| Ok(())).optional()?.is_some())
}

/// Grava uma ideia recebida. Repetir com o mesmo id não duplica.
pub fn inbox_insert(c: &Connection, id: &str, kind: &str, mime: &str, body: &[u8], created_at: i64, received_at: i64) -> Result<(), DbError> {
    c.execute(
        "INSERT OR IGNORE INTO inbox(id,kind,mime,body,created_at,received_at,status) VALUES(?1,?2,?3,?4,?5,?6,'nova')",
        params![id, kind, mime, body, created_at, received_at],
    )?;
    Ok(())
}

pub fn inbox_list(c: &Connection) -> Result<Vec<InboxItem>, DbError> {
    let mut st = c.prepare(
        "SELECT id,kind,mime,length(body),created_at,status, CASE WHEN kind='text' THEN body END
         FROM inbox ORDER BY created_at DESC, rowid DESC",
    )?;
    let v = st
        .query_map([], |r| {
            let text: Option<Vec<u8>> = r.get(6)?;
            Ok(InboxItem {
                id: r.get(0)?,
                kind: r.get(1)?,
                mime: r.get(2)?,
                size: r.get(3)?,
                created_at: r.get(4)?,
                status: r.get(5)?,
                text: text.map(|b| String::from_utf8_lossy(&b).into_owned()),
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

pub fn inbox_body(c: &Connection, id: &str) -> Result<Vec<u8>, DbError> {
    c.query_row("SELECT body FROM inbox WHERE id=?1", [id], |r| r.get(0))
        .optional()?
        .ok_or(DbError::NotFound)
}

pub fn inbox_set_status(c: &Connection, id: &str, status: &str) -> Result<(), DbError> {
    if !matches!(status, "nova" | "arquivada") {
        return Err(DbError::Invalid(format!("status {status}")));
    }
    let n = c.execute("UPDATE inbox SET status=?1 WHERE id=?2", params![status, id])?;
    if n == 0 { Err(DbError::NotFound) } else { Ok(()) }
}

pub fn inbox_delete(c: &Connection, id: &str) -> Result<(), DbError> {
    c.execute("DELETE FROM inbox WHERE id=?1", [id])?;
    Ok(())
}

fn count_words(s: &str) -> i64 {
    s.split(|ch: char| !(ch.is_alphanumeric() || ch == '-' || ch == '\'' || ch == '’'))
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count() as i64
}

/// Transforma uma ideia de texto numa cena nova: um parágrafo por linha não vazia.
pub fn inbox_to_scene(c: &Connection, id: &str, chapter_id: &str, index: Option<i64>, now: i64) -> Result<Scene, DbError> {
    let (kind, body): (String, Vec<u8>) = c
        .query_row("SELECT kind, body FROM inbox WHERE id=?1", [id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?
        .ok_or(DbError::NotFound)?;
    if kind != "text" {
        return Err(DbError::Invalid("NOT_TEXT".into()));
    }
    let text = String::from_utf8_lossy(&body).into_owned();
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let title: String = lines.first().map(|l| l.chars().take(60).collect()).unwrap_or_else(|| "Ideia".into());
    let doc = serde_json::json!({
        "type": "doc",
        "content": lines.iter().map(|l| serde_json::json!({"type": "paragraph", "content": [{"type": "text", "text": l}]})).collect::<Vec<_>>(),
    });
    let tx = super::Atomic::begin(c)?;
    let scene = scenes::scene_create(c, chapter_id, &title, index, now)?;
    scenes::scene_save(c, &scene.id, &doc.to_string(), count_words(&text), now)?;
    inbox_set_status(c, id, "arquivada")?;
    let s = scenes::scene_get(c, &scene.id)?;
    tx.commit()?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{books, test_conn};

    #[test]
    fn insert_list_and_body() {
        let (_d, c) = test_conn();
        inbox_insert(&c, "e1", "text", "text/plain", "Oi\nMarina".as_bytes(), 10, 20).unwrap();
        inbox_insert(&c, "e2", "audio", "audio/mp4", &[1, 2, 3], 11, 21).unwrap();
        let l = inbox_list(&c).unwrap();
        assert_eq!(l.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), ["e2", "e1"]); // mais nova primeiro
        assert_eq!(l[1].text.as_deref(), Some("Oi\nMarina"));
        assert_eq!(l[0].text, None);
        assert_eq!(l[0].size, 3);
        assert_eq!(inbox_body(&c, "e2").unwrap(), vec![1, 2, 3]);
        assert!(inbox_has(&c, "e1").unwrap());
    }

    #[test]
    fn insert_twice_keeps_one() {
        let (_d, c) = test_conn();
        inbox_insert(&c, "e1", "text", "text/plain", b"a", 1, 1).unwrap();
        inbox_insert(&c, "e1", "text", "text/plain", b"a", 1, 1).unwrap();
        assert_eq!(inbox_list(&c).unwrap().len(), 1);
    }

    #[test]
    fn archive_and_delete() {
        let (_d, c) = test_conn();
        inbox_insert(&c, "e1", "text", "text/plain", b"a", 1, 1).unwrap();
        inbox_set_status(&c, "e1", "arquivada").unwrap();
        assert_eq!(inbox_list(&c).unwrap()[0].status, "arquivada");
        assert!(matches!(inbox_set_status(&c, "e1", "lixo"), Err(DbError::Invalid(_))));
        inbox_delete(&c, "e1").unwrap();
        assert!(inbox_list(&c).unwrap().is_empty());
    }

    #[test]
    fn text_idea_becomes_scene_with_one_paragraph_per_line() {
        let (_d, c) = test_conn();
        let b = books::book_create(&c, "L", "conselheira", 0).unwrap();
        let ch = books::book_outline(&c, &b.id).unwrap().chapters[0].chapter.id.clone();
        inbox_insert(&c, "e1", "text", "text/plain", "E se ela souber?\n\nO irmão sabia.\nFim".as_bytes(), 1, 1).unwrap();
        let s = inbox_to_scene(&c, "e1", &ch, Some(0), 5).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&s.content_json).unwrap();
        let paras: Vec<&str> = doc["content"].as_array().unwrap().iter().map(|p| p["content"][0]["text"].as_str().unwrap()).collect();
        assert_eq!(paras, ["E se ela souber?", "O irmão sabia.", "Fim"]);
        assert_eq!(s.word_count, 8);
        assert_eq!(s.position, 0);
        assert_eq!(scenes::scene_get(&c, &s.id).unwrap().title, "E se ela souber?");
        assert_eq!(inbox_list(&c).unwrap()[0].status, "arquivada");
    }

    #[test]
    fn audio_cannot_become_scene() {
        let (_d, c) = test_conn();
        let b = books::book_create(&c, "L", "conselheira", 0).unwrap();
        let ch = books::book_outline(&c, &b.id).unwrap().chapters[0].chapter.id.clone();
        inbox_insert(&c, "e2", "audio", "audio/mp4", &[1], 1, 1).unwrap();
        assert!(matches!(inbox_to_scene(&c, "e2", &ch, None, 1), Err(DbError::Invalid(m)) if m == "NOT_TEXT"));
    }
}
