//! Mapa de ideias: cartões soltos ligados por setas, um mapa por livro.

use super::ai::{self, BibleEntry};
use super::{new_id, DbError};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

/// Cores de cartão, na ordem do menu. A primeira é o padrão.
pub const COLORS: [&str; 5] = ["papel", "amarelo", "verde", "azul", "rosa"];

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MapCard {
    pub id: String,
    pub book_id: String,
    pub text: String,
    pub color: String,
    pub x: f64,
    pub y: f64,
    pub bible_entry_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MapLink {
    pub id: String,
    pub book_id: String,
    pub from_card: String,
    pub to_card: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MapData {
    pub cards: Vec<MapCard>,
    pub links: Vec<MapLink>,
}

const CARD_COLS: &str = "id,book_id,text,color,x,y,bible_entry_id,created_at,updated_at";
const LINK_COLS: &str = "id,book_id,from_card,to_card,label";

fn card_row(r: &rusqlite::Row) -> rusqlite::Result<MapCard> {
    Ok(MapCard {
        id: r.get(0)?,
        book_id: r.get(1)?,
        text: r.get(2)?,
        color: r.get(3)?,
        x: r.get(4)?,
        y: r.get(5)?,
        bible_entry_id: r.get(6)?,
        created_at: r.get(7)?,
        updated_at: r.get(8)?,
    })
}

fn link_row(r: &rusqlite::Row) -> rusqlite::Result<MapLink> {
    Ok(MapLink { id: r.get(0)?, book_id: r.get(1)?, from_card: r.get(2)?, to_card: r.get(3)?, label: r.get(4)? })
}

fn check_color(color: &str) -> Result<(), DbError> {
    if COLORS.contains(&color) { Ok(()) } else { Err(DbError::Invalid(format!("cor {color}"))) }
}

fn affected(n: usize) -> Result<(), DbError> {
    if n == 0 { Err(DbError::NotFound) } else { Ok(()) }
}

pub fn card_get(c: &Connection, id: &str) -> Result<MapCard, DbError> {
    c.query_row(&format!("SELECT {CARD_COLS} FROM map_cards WHERE id=?1"), [id], card_row).optional()?.ok_or(DbError::NotFound)
}

pub fn map_get(c: &Connection, book_id: &str) -> Result<MapData, DbError> {
    let mut st = c.prepare(&format!("SELECT {CARD_COLS} FROM map_cards WHERE book_id=?1 ORDER BY created_at, rowid"))?;
    let cards = st.query_map([book_id], card_row)?.collect::<Result<_, _>>()?;
    let mut st = c.prepare(&format!("SELECT {LINK_COLS} FROM map_links WHERE book_id=?1 ORDER BY rowid"))?;
    let links = st.query_map([book_id], link_row)?.collect::<Result<_, _>>()?;
    Ok(MapData { cards, links })
}

pub fn card_create(c: &Connection, book_id: &str, text: &str, color: &str, x: f64, y: f64, now: i64) -> Result<MapCard, DbError> {
    check_color(color)?;
    let id = new_id();
    c.execute(
        "INSERT INTO map_cards(id,book_id,text,color,x,y,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?7)",
        params![id, book_id, text, color, x, y, now],
    )?;
    card_get(c, &id)
}

pub fn card_update(c: &Connection, id: &str, text: &str, color: &str, now: i64) -> Result<MapCard, DbError> {
    check_color(color)?;
    affected(c.execute("UPDATE map_cards SET text=?2, color=?3, updated_at=?4 WHERE id=?1", params![id, text, color, now])?)?;
    card_get(c, id)
}

pub fn card_move(c: &Connection, id: &str, x: f64, y: f64, now: i64) -> Result<(), DbError> {
    affected(c.execute("UPDATE map_cards SET x=?2, y=?3, updated_at=?4 WHERE id=?1", params![id, x, y, now])?)
}

/// Apaga o cartão; as setas dele caem junto.
pub fn card_delete(c: &Connection, id: &str) -> Result<(), DbError> {
    c.execute("DELETE FROM map_cards WHERE id=?1", [id])?;
    Ok(())
}

fn card_in_book(c: &Connection, book_id: &str, id: &str) -> Result<bool, DbError> {
    Ok(c.query_row("SELECT 1 FROM map_cards WHERE id=?1 AND book_id=?2", [id, book_id], |_| Ok(())).optional()?.is_some())
}

/// Liga dois cartões do mesmo livro. Seta repetida devolve a que já existe.
pub fn link_create(c: &Connection, book_id: &str, from: &str, to: &str) -> Result<MapLink, DbError> {
    if from == to {
        return Err(DbError::Invalid("um cartão não liga nele mesmo".into()));
    }
    if !card_in_book(c, book_id, from)? || !card_in_book(c, book_id, to)? {
        return Err(DbError::Invalid("cartão de outro livro ou inexistente".into()));
    }
    let existing = c
        .query_row(&format!("SELECT {LINK_COLS} FROM map_links WHERE from_card=?1 AND to_card=?2"), [from, to], link_row)
        .optional()?;
    if let Some(l) = existing {
        return Ok(l);
    }
    let id = new_id();
    c.execute("INSERT INTO map_links(id,book_id,from_card,to_card) VALUES(?1,?2,?3,?4)", params![id, book_id, from, to])?;
    Ok(MapLink { id, book_id: book_id.into(), from_card: from.into(), to_card: to.into(), label: String::new() })
}

pub fn link_set_label(c: &Connection, id: &str, label: &str) -> Result<(), DbError> {
    affected(c.execute("UPDATE map_links SET label=?2 WHERE id=?1", params![id, label.trim()])?)
}

pub fn link_delete(c: &Connection, id: &str) -> Result<(), DbError> {
    c.execute("DELETE FROM map_links WHERE id=?1", [id])?;
    Ok(())
}

/// Leva o cartão para a ficha do livro: 1ª linha é o nome, o resto a descrição.
/// Se o cartão já virou ficha, ou já existe ficha com esse nome, usa a que existe.
pub fn card_to_bible(c: &Connection, card_id: &str, kind: &str, now: i64) -> Result<BibleEntry, DbError> {
    let card = card_get(c, card_id)?;
    if let Some(e) = card.bible_entry_id.as_deref().and_then(|id| ai::bible_get(c, id).ok()) {
        return Ok(e);
    }
    let mut lines = card.text.trim().splitn(2, '\n');
    let name = lines.next().unwrap_or("").trim().to_string();
    let body = lines.next().unwrap_or("").trim().to_string();
    if name.is_empty() {
        return Err(DbError::Invalid("cartão vazio".into()));
    }
    let entry = match ai::bible_find(c, &card.book_id, kind, &name)? {
        Some(e) => e,
        None => ai::bible_create_user(c, &card.book_id, kind, &name, &body, now)?,
    };
    c.execute("UPDATE map_cards SET bible_entry_id=?2, updated_at=?3 WHERE id=?1", params![card_id, entry.id, now])?;
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{ai, books, test_conn};
    use rusqlite::Connection;

    fn book(c: &Connection, t: &str) -> String {
        books::book_create(c, t, "conselheira", 0).unwrap().id
    }

    #[test]
    fn cards_and_links_roundtrip() {
        let (_d, c) = test_conn();
        let b = book(&c, "L");
        let a = card_create(&c, &b, "Marina", "papel", 10.0, 20.0, 1).unwrap();
        let z = card_create(&c, &b, "A carta some", "amarelo", 300.0, 20.0, 1).unwrap();
        let l = link_create(&c, &b, &a.id, &z.id).unwrap();
        assert_eq!(link_create(&c, &b, &a.id, &z.id).unwrap().id, l.id, "seta repetida devolve a existente");
        link_set_label(&c, &l.id, "por quê?").unwrap();
        card_move(&c, &a.id, 50.0, 60.0, 2).unwrap();
        let u = card_update(&c, &z.id, "A carta some\nno cap. 3", "verde", 3).unwrap();
        assert_eq!(u.color, "verde");
        let m = map_get(&c, &b).unwrap();
        assert_eq!(m.cards.len(), 2);
        assert_eq!(m.links.len(), 1);
        assert_eq!(m.links[0].label, "por quê?");
        let moved = m.cards.iter().find(|x| x.id == a.id).unwrap();
        assert_eq!((moved.x, moved.y), (50.0, 60.0));
        card_delete(&c, &a.id).unwrap();
        let m = map_get(&c, &b).unwrap();
        assert_eq!(m.cards.len(), 1);
        assert!(m.links.is_empty());
    }

    #[test]
    fn map_is_per_book() {
        let (_d, c) = test_conn();
        let (b1, b2) = (book(&c, "A"), book(&c, "B"));
        card_create(&c, &b1, "só no A", "papel", 0.0, 0.0, 1).unwrap();
        assert!(map_get(&c, &b2).unwrap().cards.is_empty());
    }

    #[test]
    fn link_rejects_card_from_other_book() {
        let (_d, c) = test_conn();
        let (b1, b2) = (book(&c, "A"), book(&c, "B"));
        let a = card_create(&c, &b1, "a", "papel", 0.0, 0.0, 1).unwrap();
        let z = card_create(&c, &b2, "z", "papel", 0.0, 0.0, 1).unwrap();
        assert!(matches!(link_create(&c, &b1, &a.id, &z.id), Err(DbError::Invalid(_))));
    }

    #[test]
    fn link_rejects_self() {
        let (_d, c) = test_conn();
        let b = book(&c, "A");
        let a = card_create(&c, &b, "a", "papel", 0.0, 0.0, 1).unwrap();
        assert!(matches!(link_create(&c, &b, &a.id, &a.id), Err(DbError::Invalid(_))));
    }

    #[test]
    fn invalid_color_rejected() {
        let (_d, c) = test_conn();
        let b = book(&c, "A");
        assert!(matches!(card_create(&c, &b, "a", "roxo", 0.0, 0.0, 1), Err(DbError::Invalid(_))));
        let a = card_create(&c, &b, "a", "papel", 0.0, 0.0, 1).unwrap();
        assert!(matches!(card_update(&c, &a.id, "a", "roxo", 1), Err(DbError::Invalid(_))));
    }

    #[test]
    fn card_to_bible_splits_first_line_and_is_idempotent() {
        let (_d, c) = test_conn();
        let b = book(&c, "A");
        let a = card_create(&c, &b, "  Marina \nFaroleira, 30 anos", "papel", 0.0, 0.0, 1).unwrap();
        let e = card_to_bible(&c, &a.id, "personagem", 2).unwrap();
        assert_eq!((e.name.as_str(), e.body.as_str(), e.kind.as_str()), ("Marina", "Faroleira, 30 anos", "personagem"));
        assert!(e.edited_by_user);
        let again = card_to_bible(&c, &a.id, "lugar", 3).unwrap();
        assert_eq!(again.id, e.id);
        assert_eq!(ai::bible_list(&c, &b).unwrap().len(), 1);
        assert_eq!(map_get(&c, &b).unwrap().cards[0].bible_entry_id.as_deref(), Some(e.id.as_str()));
    }

    #[test]
    fn card_to_bible_reuses_existing_entry_with_same_name() {
        let (_d, c) = test_conn();
        let b = book(&c, "A");
        let old = ai::bible_upsert_ai(&c, &b, "personagem", "Marina", &[], "Faroleira", None, None, 1).unwrap();
        let a = card_create(&c, &b, "marina", "papel", 0.0, 0.0, 1).unwrap();
        assert_eq!(card_to_bible(&c, &a.id, "personagem", 2).unwrap().id, old.id);
        assert_eq!(ai::bible_list(&c, &b).unwrap().len(), 1);
    }

    #[test]
    fn deleting_bible_entry_clears_card_link() {
        let (_d, c) = test_conn();
        let b = book(&c, "A");
        let a = card_create(&c, &b, "Farol", "papel", 0.0, 0.0, 1).unwrap();
        let e = card_to_bible(&c, &a.id, "lugar", 2).unwrap();
        ai::bible_delete(&c, &e.id).unwrap();
        assert_eq!(map_get(&c, &b).unwrap().cards[0].bible_entry_id, None);
    }
}
