//! Imagens do texto e da ficha do livro, guardadas cifradas no banco.

use super::DbError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BookMeta {
    pub author: Option<String>,
    pub synopsis: Option<String>,
    pub dedication: Option<String>,
    pub cover_image_id: Option<String>,
    pub back_image_id: Option<String>,
}

use rusqlite::{params, OptionalExtension};
use std::collections::HashSet;

pub fn image_add(c: &Connection, book_id: &str, mime: &str, bytes: &[u8], w: i64, h: i64, now: i64) -> Result<String, DbError> {
    if !mime.starts_with("image/") {
        return Err(DbError::Invalid("NOT_IMAGE".into()));
    }
    let id = super::new_id();
    c.execute(
        "INSERT INTO images(id,book_id,mime,bytes,width,height,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![id, book_id, mime, bytes, w, h, now],
    )?;
    Ok(id)
}

pub fn image_get(c: &Connection, id: &str) -> Result<(String, Vec<u8>), DbError> {
    c.query_row("SELECT mime, bytes FROM images WHERE id=?1", [id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?
        .ok_or(DbError::NotFound)
}

/// Só o tipo da imagem (sem ler os bytes).
pub fn image_mime(c: &Connection, id: &str) -> Result<String, DbError> {
    c.query_row("SELECT mime FROM images WHERE id=?1", [id], |r| r.get(0)).optional()?.ok_or(DbError::NotFound)
}

/// Ids citados como `lume-img:<id>` num texto (JSON de cena ou de versão).
fn refs_in(text: &str, out: &mut HashSet<String>) {
    for part in text.split("lume-img:").skip(1) {
        let id: String = part.chars().take_while(|c| c.is_ascii_hexdigit() || *c == '-').collect();
        if !id.is_empty() {
            out.insert(id);
        }
    }
}

/// Apaga imagens do livro que ninguém usa: nem as cenas, nem as versões antigas, nem as fichas.
/// O "em uso" olha TODOS os livros: uma figura copiada de um livro e colada em outro continua viva.
pub fn images_gc(c: &Connection, book_id: &str) -> Result<usize, DbError> {
    let mut used = HashSet::new();
    let mut st = c.prepare("SELECT content_json FROM scenes WHERE content_json LIKE '%lume-img:%' UNION ALL SELECT content_json FROM scene_versions WHERE content_json LIKE '%lume-img:%'")?;
    for json in st.query_map([], |r| r.get::<_, String>(0))? {
        refs_in(&json?, &mut used);
    }
    let mut st = c.prepare("SELECT cover_image_id FROM books WHERE cover_image_id IS NOT NULL UNION ALL SELECT back_image_id FROM books WHERE back_image_id IS NOT NULL")?;
    for id in st.query_map([], |r| r.get::<_, String>(0))? {
        used.insert(id?);
    }
    let ids: Vec<String> = c
        .prepare("SELECT id FROM images WHERE book_id=?1")?
        .query_map([book_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut n = 0;
    for id in ids.into_iter().filter(|id| !used.contains(id)) {
        n += c.execute("DELETE FROM images WHERE id=?1", [&id])?;
    }
    Ok(n)
}

pub fn book_meta_get(c: &Connection, book_id: &str) -> Result<BookMeta, DbError> {
    c.query_row(
        "SELECT author, synopsis, dedication, cover_image_id, back_image_id FROM books WHERE id=?1",
        [book_id],
        |r| Ok(BookMeta { author: r.get(0)?, synopsis: r.get(1)?, dedication: r.get(2)?, cover_image_id: r.get(3)?, back_image_id: r.get(4)? }),
    )
    .optional()?
    .ok_or(DbError::NotFound)
}

pub fn book_meta_set(c: &Connection, book_id: &str, m: &BookMeta) -> Result<(), DbError> {
    let blank = |s: &Option<String>| s.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    let n = c.execute(
        "UPDATE books SET author=?1, synopsis=?2, dedication=?3, cover_image_id=?4, back_image_id=?5 WHERE id=?6",
        params![blank(&m.author), blank(&m.synopsis), blank(&m.dedication), m.cover_image_id, m.back_image_id, book_id],
    )?;
    if n == 0 { Err(DbError::NotFound) } else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{books, scenes, test_conn};

    fn book(c: &Connection) -> (String, String) {
        let b = books::book_create(c, "L", "conselheira", 0).unwrap();
        let s = books::book_outline(c, &b.id).unwrap().chapters[0].scenes[0].id.clone();
        (b.id, s)
    }

    fn doc_with(id: &str) -> String {
        format!(r#"{{"type":"doc","content":[{{"type":"figure","attrs":{{"src":"lume-img:{id}","caption":"x"}}}}]}}"#)
    }

    #[test]
    fn add_and_get_round_trip() {
        let (_d, c) = test_conn();
        let (b, _) = book(&c);
        let id = image_add(&c, &b, "image/png", &[1, 2, 3], 10, 20, 1).unwrap();
        assert_eq!(image_get(&c, &id).unwrap(), ("image/png".to_string(), vec![1, 2, 3]));
        assert!(matches!(image_get(&c, "nao-existe"), Err(DbError::NotFound)));
        assert_eq!(image_mime(&c, &id).unwrap(), "image/png");
        assert!(matches!(image_mime(&c, "nao-existe"), Err(DbError::NotFound)));
    }

    #[test]
    fn gc_removes_unreferenced_and_keeps_used_ones() {
        let (_d, c) = test_conn();
        let (b, s) = book(&c);
        let used = image_add(&c, &b, "image/png", &[1], 1, 1, 1).unwrap();
        let loose = image_add(&c, &b, "image/png", &[2], 1, 1, 1).unwrap();
        scenes::scene_save(&c, &s, &doc_with(&used), 0, 1).unwrap();
        assert_eq!(images_gc(&c, &b).unwrap(), 1);
        assert!(image_get(&c, &used).is_ok());
        assert!(image_get(&c, &loose).is_err());
    }

    #[test]
    fn gc_keeps_image_of_book_a_pasted_into_book_b() {
        let (_d, c) = test_conn();
        let (a, sa) = book(&c);
        let (b, sb) = book(&c);
        let img = image_add(&c, &a, "image/png", &[1], 1, 1, 1).unwrap();
        // a figura foi copiada do livro A e colada no B; depois saiu do A
        scenes::scene_save(&c, &sb, &doc_with(&img), 0, 1).unwrap();
        scenes::scene_save(&c, &sa, r#"{"type":"doc","content":[]}"#, 0, 1).unwrap();
        assert_eq!(images_gc(&c, &a).unwrap(), 0);
        assert!(image_get(&c, &img).is_ok(), "o livro B ainda usa a imagem");
        let _ = b;
    }

    #[test]
    fn gc_keeps_image_only_used_by_an_old_version() {
        let (_d, c) = test_conn();
        let (b, s) = book(&c);
        let img = image_add(&c, &b, "image/png", &[1], 1, 1, 1).unwrap();
        scenes::scene_save(&c, &s, &doc_with(&img), 0, 1).unwrap(); // vira versão
        scenes::scene_save(&c, &s, r#"{"type":"doc","content":[]}"#, 0, 2).unwrap(); // tirou do texto
        images_gc(&c, &b).unwrap();
        assert!(image_get(&c, &img).is_ok(), "restaurar a versão antiga precisa da imagem");
    }

    #[test]
    fn gc_keeps_cover_and_back_cover() {
        let (_d, c) = test_conn();
        let (b, _) = book(&c);
        let cover = image_add(&c, &b, "image/png", &[1], 1, 1, 1).unwrap();
        let back = image_add(&c, &b, "image/jpeg", &[2], 1, 1, 1).unwrap();
        book_meta_set(&c, &b, &BookMeta { cover_image_id: Some(cover.clone()), back_image_id: Some(back.clone()), ..Default::default() }).unwrap();
        assert_eq!(images_gc(&c, &b).unwrap(), 0);
    }

    #[test]
    fn gc_does_not_touch_other_books() {
        let (_d, c) = test_conn();
        let (b1, _) = book(&c);
        let (b2, _) = book(&c);
        let other = image_add(&c, &b2, "image/png", &[1], 1, 1, 1).unwrap();
        images_gc(&c, &b1).unwrap();
        assert!(image_get(&c, &other).is_ok());
    }

    #[test]
    fn meta_get_and_set() {
        let (_d, c) = test_conn();
        let (b, _) = book(&c);
        assert_eq!(book_meta_get(&c, &b).unwrap(), BookMeta::default());
        let m = BookMeta { author: Some("Hefesto".into()), synopsis: Some("João…".into()), dedication: Some("Para a Marina".into()), ..Default::default() };
        book_meta_set(&c, &b, &m).unwrap();
        assert_eq!(book_meta_get(&c, &b).unwrap(), m);
        assert!(matches!(book_meta_get(&c, "x"), Err(DbError::NotFound)));
    }
}
