use super::DbError;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Book {
    pub id: String,
    pub title: String,
    pub ai_mode: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Chapter {
    pub id: String,
    pub book_id: String,
    pub title: String,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SceneSummary {
    pub id: String,
    pub chapter_id: String,
    pub title: String,
    pub word_count: i64,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ChapterOutline {
    pub chapter: Chapter,
    pub scenes: Vec<SceneSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Outline {
    pub book: Book,
    pub chapters: Vec<ChapterOutline>,
}

use super::{insert_clamped, new_id, ordered_ids, write_positions};
use rusqlite::{params, OptionalExtension};

fn check_mode(m: &str) -> Result<(), DbError> {
    match m {
        "conselheira" | "coautora" => Ok(()),
        _ => Err(DbError::Invalid(format!("ai_mode {m}"))),
    }
}

fn affected(n: usize) -> Result<(), DbError> {
    if n == 0 { Err(DbError::NotFound) } else { Ok(()) }
}

fn book_row(r: &rusqlite::Row) -> rusqlite::Result<Book> {
    Ok(Book { id: r.get(0)?, title: r.get(1)?, ai_mode: r.get(2)?, created_at: r.get(3)? })
}

fn chapter_row(r: &rusqlite::Row) -> rusqlite::Result<Chapter> {
    Ok(Chapter { id: r.get(0)?, book_id: r.get(1)?, title: r.get(2)?, position: r.get(3)? })
}

pub fn book_list(c: &Connection) -> Result<Vec<Book>, DbError> {
    let mut st = c.prepare("SELECT id,title,ai_mode,created_at FROM books ORDER BY created_at, title")?;
    let v = st.query_map([], book_row)?.collect::<Result<_, _>>()?;
    Ok(v)
}

pub fn book_create(c: &Connection, title: &str, ai_mode: &str, now: i64) -> Result<Book, DbError> {
    check_mode(ai_mode)?;
    let tx = super::Atomic::begin(c)?;
    let b = Book { id: new_id(), title: title.into(), ai_mode: ai_mode.into(), created_at: now };
    c.execute(
        "INSERT INTO books(id,title,ai_mode,created_at) VALUES(?1,?2,?3,?4)",
        params![b.id, b.title, b.ai_mode, b.created_at],
    )?;
    let ch = chapter_create(c, &b.id, "Capítulo 1", None)?;
    super::scenes::scene_create(c, &ch.id, "Cena 1", None, now)?;
    tx.commit()?;
    Ok(b)
}

pub fn book_rename(c: &Connection, id: &str, title: &str) -> Result<(), DbError> {
    affected(c.execute("UPDATE books SET title=?1 WHERE id=?2", params![title, id])?)
}

pub fn book_set_mode(c: &Connection, id: &str, ai_mode: &str) -> Result<(), DbError> {
    check_mode(ai_mode)?;
    affected(c.execute("UPDATE books SET ai_mode=?1 WHERE id=?2", params![ai_mode, id])?)
}

pub fn book_outline(c: &Connection, book_id: &str) -> Result<Outline, DbError> {
    let book = c
        .query_row("SELECT id,title,ai_mode,created_at FROM books WHERE id=?1", [book_id], book_row)
        .optional()?
        .ok_or(DbError::NotFound)?;
    let mut st = c.prepare("SELECT id,book_id,title,position FROM chapters WHERE book_id=?1 ORDER BY position")?;
    let chapters: Vec<Chapter> = st.query_map([book_id], chapter_row)?.collect::<Result<_, _>>()?;
    let mut sc = c.prepare(
        "SELECT id,chapter_id,title,word_count,position FROM scenes WHERE chapter_id=?1 ORDER BY position",
    )?;
    let mut out = Vec::with_capacity(chapters.len());
    for chapter in chapters {
        let scenes = sc
            .query_map([&chapter.id], |r| {
                Ok(SceneSummary {
                    id: r.get(0)?,
                    chapter_id: r.get(1)?,
                    title: r.get(2)?,
                    word_count: r.get(3)?,
                    position: r.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        out.push(ChapterOutline { chapter, scenes });
    }
    Ok(Outline { book, chapters: out })
}

pub fn chapter_create(c: &Connection, book_id: &str, title: &str, index: Option<i64>) -> Result<Chapter, DbError> {
    let tx = super::Atomic::begin(c)?;
    let id = new_id();
    let mut ids = ordered_ids(c, "chapters", "book_id", book_id)?;
    c.execute(
        "INSERT INTO chapters(id,book_id,title,position) VALUES(?1,?2,?3,-1)",
        params![id, book_id, title],
    )?;
    insert_clamped(&mut ids, id.clone(), index);
    write_positions(c, "chapters", &ids)?;
    let ch = c.query_row("SELECT id,book_id,title,position FROM chapters WHERE id=?1", [&id], chapter_row)?;
    tx.commit()?;
    Ok(ch)
}

pub fn chapter_rename(c: &Connection, id: &str, title: &str) -> Result<(), DbError> {
    affected(c.execute("UPDATE chapters SET title=?1 WHERE id=?2", params![title, id])?)
}

fn chapter_book(c: &Connection, id: &str) -> Result<String, DbError> {
    c.query_row("SELECT book_id FROM chapters WHERE id=?1", [id], |r| r.get(0))
        .optional()?
        .ok_or(DbError::NotFound)
}

pub fn chapter_move(c: &Connection, id: &str, to_index: i64) -> Result<(), DbError> {
    let tx = super::Atomic::begin(c)?;
    let book = chapter_book(c, id)?;
    let mut ids = ordered_ids(c, "chapters", "book_id", &book)?;
    ids.retain(|x| x != id);
    insert_clamped(&mut ids, id.to_string(), Some(to_index));
    write_positions(c, "chapters", &ids)?;
    tx.commit()?;
    Ok(())
}

pub fn chapter_delete(c: &Connection, id: &str) -> Result<(), DbError> {
    let tx = super::Atomic::begin(c)?;
    let book = chapter_book(c, id)?;
    let n: i64 = c.query_row("SELECT count(*) FROM scenes WHERE chapter_id=?1", [id], |r| r.get(0))?;
    if n > 0 {
        return Err(DbError::ChapterNotEmpty);
    }
    c.execute("DELETE FROM chapters WHERE id=?1", [id])?;
    write_positions(c, "chapters", &ordered_ids(c, "chapters", "book_id", &book)?)?;
    tx.commit()?;
    Ok(())
}
