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

// ---------- propostas da IA ----------

/// Cartões ou nome sugeridos pela IA, esperando a pessoa decidir.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MapProposal {
    pub id: String,
    pub book_id: String,
    pub conversation_id: Option<String>,
    /// "cartoes" ou "nome".
    pub kind: String,
    pub payload_json: String,
    pub status: String,
    pub created_at: i64,
}

const PROP_COLS: &str = "id,book_id,conversation_id,kind,payload_json,status,created_at";
/// Máximo de cartões numa proposta.
pub const MAX_PROPOSED: usize = 12;
const MAX_CARD_TEXT: usize = 500;

fn prop_row(r: &rusqlite::Row) -> rusqlite::Result<MapProposal> {
    Ok(MapProposal { id: r.get(0)?, book_id: r.get(1)?, conversation_id: r.get(2)?, kind: r.get(3)?, payload_json: r.get(4)?, status: r.get(5)?, created_at: r.get(6)? })
}

fn str_of<'a>(v: &'a serde_json::Value, k: &str) -> &'a str {
    v.get(k).and_then(serde_json::Value::as_str).map(str::trim).unwrap_or("")
}

fn invalid(m: impl Into<String>) -> DbError {
    DbError::Invalid(m.into())
}

fn insert_proposal(c: &Connection, book_id: &str, conv_id: Option<&str>, kind: &str, payload: &serde_json::Value, now: i64) -> Result<MapProposal, DbError> {
    let id = new_id();
    c.execute(
        "INSERT INTO map_proposals(id,book_id,conversation_id,kind,payload_json,status,created_at) VALUES(?1,?2,?3,?4,?5,'pendente',?6)",
        params![id, book_id, conv_id, kind, payload.to_string(), now],
    )?;
    proposal_get(c, &id)
}

pub fn proposal_get(c: &Connection, id: &str) -> Result<MapProposal, DbError> {
    c.query_row(&format!("SELECT {PROP_COLS} FROM map_proposals WHERE id=?1"), [id], prop_row).optional()?.ok_or(DbError::NotFound)
}

/// Valida e guarda uma proposta de cartões. Payload:
/// `{"cartoes":[{"ref","texto"}],"setas":[{"de","para","rotulo"?}]}`; `de`/`para` é um ref novo
/// ou o id de um cartão deste livro.
pub fn proposal_add_cards(c: &Connection, book_id: &str, conv_id: Option<&str>, payload: &serde_json::Value, now: i64) -> Result<MapProposal, DbError> {
    let empty = vec![];
    let cards = payload.get("cartoes").and_then(serde_json::Value::as_array).unwrap_or(&empty);
    if cards.is_empty() || cards.len() > MAX_PROPOSED {
        return Err(invalid(format!("proponha de 1 a {MAX_PROPOSED} cartões")));
    }
    let mut refs = std::collections::HashSet::new();
    let mut clean_cards = vec![];
    for k in cards {
        let (r, t) = (str_of(k, "ref"), str_of(k, "texto"));
        if r.is_empty() || t.is_empty() {
            return Err(invalid("todo cartão precisa de 'ref' e 'texto'"));
        }
        if t.chars().count() > MAX_CARD_TEXT {
            return Err(invalid(format!("texto de cartão com mais de {MAX_CARD_TEXT} caracteres")));
        }
        if !refs.insert(r.to_string()) {
            return Err(invalid(format!("ref repetido: {r}")));
        }
        clean_cards.push(serde_json::json!({"ref": r, "texto": t}));
    }
    let mut clean_links = vec![];
    for l in payload.get("setas").and_then(serde_json::Value::as_array).unwrap_or(&empty) {
        let (a, z) = (str_of(l, "de"), str_of(l, "para"));
        for end in [a, z] {
            if !refs.contains(end) && !card_in_book(c, book_id, end)? {
                return Err(invalid(format!("a seta cita '{end}', que não é um cartão novo nem um cartão do mapa")));
            }
        }
        if a == z {
            return Err(invalid("seta de um cartão para ele mesmo"));
        }
        let mut link = serde_json::json!({"de": a, "para": z, "rotulo": str_of(l, "rotulo")});
        // seta para um cartão que já está no mapa: guarda o título dele para a pessoa ver o que aprova
        for (end, key) in [(a, "de_texto"), (z, "para_texto")] {
            if !refs.contains(end) {
                let t = card_get(c, end)?.text;
                link[key] = serde_json::Value::from(t.trim().lines().next().unwrap_or("").trim());
            }
        }
        clean_links.push(link);
    }
    insert_proposal(c, book_id, conv_id, "cartoes", &serde_json::json!({"cartoes": clean_cards, "setas": clean_links}), now)
}

pub fn proposal_add_name(c: &Connection, book_id: &str, conv_id: Option<&str>, nome: &str, now: i64) -> Result<MapProposal, DbError> {
    let nome = nome.trim();
    if nome.is_empty() {
        return Err(invalid("nome vazio"));
    }
    insert_proposal(c, book_id, conv_id, "nome", &serde_json::json!({"nome": nome}), now)
}

pub fn proposals_pending(c: &Connection, book_id: &str) -> Result<Vec<MapProposal>, DbError> {
    let mut st = c.prepare(&format!("SELECT {PROP_COLS} FROM map_proposals WHERE book_id=?1 AND status='pendente' ORDER BY created_at, rowid"))?;
    let v = st.query_map([book_id], prop_row)?.collect::<Result<_, _>>()?;
    Ok(v)
}

/// Aceita a proposta. Cartões: cria os `refs` escolhidos em grade de 2 colunas à direita do mapa e as setas
/// cujos dois lados existem. Nome: renomeia o livro (deixa de ser provisório).
pub fn proposal_accept(c: &Connection, id: &str, refs: &[String], now: i64) -> Result<Vec<MapCard>, DbError> {
    let p = proposal_get(c, id)?;
    if p.status != "pendente" {
        return Err(invalid("proposta já resolvida"));
    }
    let payload: serde_json::Value = serde_json::from_str(&p.payload_json).map_err(|e| invalid(e.to_string()))?;
    let tx = super::Atomic::begin(c)?;
    let mut out = vec![];
    if p.kind == "nome" {
        super::books::book_rename(c, &p.book_id, str_of(&payload, "nome"))?;
    } else {
        let chosen: Vec<&serde_json::Value> = payload["cartoes"].as_array().into_iter().flatten().filter(|k| refs.iter().any(|r| r == str_of(k, "ref"))).collect();
        if chosen.is_empty() {
            return Err(invalid("escolha ao menos um cartão"));
        }
        let (ox, oy): (f64, f64) = c.query_row(
            "SELECT COALESCE(MAX(x) + 380, 0), COALESCE(MIN(y), 0) FROM map_cards WHERE book_id=?1",
            [&p.book_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let mut by_ref = std::collections::HashMap::new();
        for (i, k) in chosen.iter().enumerate() {
            let (col, row) = ((i % 2) as f64, (i / 2) as f64);
            let card = card_create(c, &p.book_id, str_of(k, "texto"), COLORS[0], ox + col * 340.0, oy + row * 200.0, now)?;
            by_ref.insert(str_of(k, "ref").to_string(), card.id.clone());
            out.push(card);
        }
        let resolve = |end: &str| -> Result<Option<String>, DbError> {
            if let Some(id) = by_ref.get(end) {
                return Ok(Some(id.clone()));
            }
            Ok(card_in_book(c, &p.book_id, end)?.then(|| end.to_string()))
        };
        for l in payload["setas"].as_array().into_iter().flatten() {
            if let (Some(a), Some(z)) = (resolve(str_of(l, "de"))?, resolve(str_of(l, "para"))?) {
                let link = link_create(c, &p.book_id, &a, &z)?;
                let label = str_of(l, "rotulo");
                if !label.is_empty() {
                    link_set_label(c, &link.id, label)?;
                }
            }
        }
    }
    c.execute("UPDATE map_proposals SET status='aceita' WHERE id=?1", [id])?;
    tx.commit()?;
    Ok(out)
}

pub fn proposal_discard(c: &Connection, id: &str) -> Result<(), DbError> {
    affected(c.execute("UPDATE map_proposals SET status='descartada' WHERE id=?1 AND status='pendente'", [id])?)
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

    fn cards_payload(cards: &[(&str, &str)], links: &[(&str, &str)]) -> serde_json::Value {
        serde_json::json!({
            "cartoes": cards.iter().map(|(r, t)| serde_json::json!({"ref": r, "texto": t})).collect::<Vec<_>>(),
            "setas": links.iter().map(|(a, z)| serde_json::json!({"de": a, "para": z})).collect::<Vec<_>>(),
        })
    }

    #[test]
    fn accept_places_cards_right_of_existing_in_grid() {
        let (_d, c) = test_conn();
        let b = book(&c, "A");
        card_create(&c, &b, "velho", "papel", 100.0, 50.0, 1).unwrap();
        let p = proposal_add_cards(&c, &b, None, &cards_payload(&[("a", "1"), ("b", "2"), ("c", "3"), ("d", "4")], &[]), 2).unwrap();
        let got = proposal_accept(&c, &p.id, &["a".into(), "b".into(), "c".into(), "d".into()], 3).unwrap();
        let pos: Vec<(f64, f64)> = got.iter().map(|k| (k.x, k.y)).collect();
        assert_eq!(pos, vec![(480.0, 50.0), (820.0, 50.0), (480.0, 250.0), (820.0, 250.0)]);
        assert_eq!(got[0].text, "1");
    }

    #[test]
    fn accept_on_empty_map_starts_at_origin() {
        let (_d, c) = test_conn();
        let b = book(&c, "A");
        let p = proposal_add_cards(&c, &b, None, &cards_payload(&[("a", "1")], &[]), 2).unwrap();
        let got = proposal_accept(&c, &p.id, &["a".into()], 3).unwrap();
        assert_eq!((got[0].x, got[0].y), (0.0, 0.0));
    }

    #[test]
    fn accept_subset_skips_links_to_unchosen() {
        let (_d, c) = test_conn();
        let b = book(&c, "A");
        let old = card_create(&c, &b, "velho", "papel", 0.0, 0.0, 1).unwrap();
        let p = proposal_add_cards(&c, &b, None, &cards_payload(&[("a", "1"), ("b", "2")], &[("a", "b"), ("a", &old.id)]), 2).unwrap();
        proposal_accept(&c, &p.id, &["a".into()], 3).unwrap();
        let m = map_get(&c, &b).unwrap();
        assert_eq!(m.cards.len(), 2);
        assert_eq!(m.links.len(), 1);
        assert_eq!(m.links[0].to_card, old.id);
    }

    #[test]
    fn accept_ignores_link_to_deleted_card() {
        let (_d, c) = test_conn();
        let b = book(&c, "A");
        let old = card_create(&c, &b, "velho", "papel", 0.0, 0.0, 1).unwrap();
        let p = proposal_add_cards(&c, &b, None, &cards_payload(&[("a", "1")], &[("a", &old.id)]), 2).unwrap();
        card_delete(&c, &old.id).unwrap();
        let got = proposal_accept(&c, &p.id, &["a".into()], 3).unwrap();
        assert_eq!(got.len(), 1);
        assert!(map_get(&c, &b).unwrap().links.is_empty());
    }

    #[test]
    fn accept_twice_fails_without_duplicates() {
        let (_d, c) = test_conn();
        let b = book(&c, "A");
        let p = proposal_add_cards(&c, &b, None, &cards_payload(&[("a", "1")], &[]), 2).unwrap();
        proposal_accept(&c, &p.id, &["a".into()], 3).unwrap();
        assert!(matches!(proposal_accept(&c, &p.id, &["a".into()], 4), Err(DbError::Invalid(_))));
        assert_eq!(map_get(&c, &b).unwrap().cards.len(), 1);
        assert!(proposals_pending(&c, &b).unwrap().is_empty());
    }

    #[test]
    fn accept_with_no_refs_fails() {
        let (_d, c) = test_conn();
        let b = book(&c, "A");
        let p = proposal_add_cards(&c, &b, None, &cards_payload(&[("a", "1")], &[]), 2).unwrap();
        assert!(matches!(proposal_accept(&c, &p.id, &[], 3), Err(DbError::Invalid(_))));
        assert_eq!(proposals_pending(&c, &b).unwrap().len(), 1);
    }

    #[test]
    fn add_cards_rejects_unknown_foreign_or_empty() {
        let (_d, c) = test_conn();
        let (b, other) = (book(&c, "A"), book(&c, "B"));
        let foreign = card_create(&c, &other, "x", "papel", 0.0, 0.0, 1).unwrap();
        for payload in [
            cards_payload(&[("a", "1")], &[("a", "nao-existe")]),
            cards_payload(&[("a", "1")], &[("a", &foreign.id)]),
            cards_payload(&[], &[]),
            cards_payload(&[("a", "  ")], &[]),
            cards_payload(&[("a", "1"), ("a", "2")], &[]),
        ] {
            assert!(matches!(proposal_add_cards(&c, &b, None, &payload, 2), Err(DbError::Invalid(_))), "{payload}");
        }
        let many: Vec<(String, String)> = (0..13).map(|i| (format!("r{i}"), "t".to_string())).collect();
        let many: Vec<(&str, &str)> = many.iter().map(|(a, z)| (a.as_str(), z.as_str())).collect();
        assert!(proposal_add_cards(&c, &b, None, &cards_payload(&many, &[]), 2).is_err());
        assert!(proposals_pending(&c, &b).unwrap().is_empty());
    }

    #[test]
    fn arrows_to_existing_cards_carry_their_text() {
        let (_d, c) = test_conn();
        let b = book(&c, "A");
        let old = card_create(&c, &b, "Marina\nfaroleira", "papel", 0.0, 0.0, 1).unwrap();
        let p = proposal_add_cards(&c, &b, None, &cards_payload(&[("a", "A carta")], &[("a", &old.id)]), 2).unwrap();
        let v: serde_json::Value = serde_json::from_str(&p.payload_json).unwrap();
        assert_eq!(v["setas"][0]["para_texto"], "Marina");
        assert!(v["setas"][0].get("de_texto").is_none());
    }

    #[test]
    fn name_proposal_renames_and_clears_provisional() {
        let (_d, c) = test_conn();
        let b = book(&c, "");
        let p = proposal_add_name(&c, &b, None, " O Farol ", 2).unwrap();
        assert!(proposal_accept(&c, &p.id, &[], 3).unwrap().is_empty());
        let o = books::book_outline(&c, &b).unwrap();
        assert_eq!(o.book.title, "O Farol");
        assert!(!o.book.title_provisional);
    }

    #[test]
    fn discard_sets_status() {
        let (_d, c) = test_conn();
        let b = book(&c, "A");
        let p = proposal_add_name(&c, &b, None, "X", 2).unwrap();
        proposal_discard(&c, &p.id).unwrap();
        assert!(proposals_pending(&c, &b).unwrap().is_empty());
        assert!(proposal_accept(&c, &p.id, &[], 3).is_err());
    }
}
