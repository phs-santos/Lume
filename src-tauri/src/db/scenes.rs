use super::DbError;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Scene {
    pub id: String,
    pub chapter_id: String,
    pub title: String,
    pub content_json: String,
    pub word_count: i64,
    pub position: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Version {
    pub id: String,
    pub created_at: i64,
    pub word_count: i64,
}

/// Intervalo mínimo entre versões automáticas: 10 minutos, em milissegundos.
pub const VERSION_INTERVAL_MS: i64 = 10 * 60 * 1000;

use super::{insert_clamped, new_id, ordered_ids, write_positions};
use rusqlite::{params, OptionalExtension};

pub const EMPTY_DOC: &str = r#"{"type":"doc","content":[{"type":"paragraph"}]}"#;

fn scene_row(r: &rusqlite::Row) -> rusqlite::Result<Scene> {
    Ok(Scene {
        id: r.get(0)?,
        chapter_id: r.get(1)?,
        title: r.get(2)?,
        content_json: r.get(3)?,
        word_count: r.get(4)?,
        position: r.get(5)?,
        updated_at: r.get(6)?,
    })
}

const SCENE_COLS: &str = "id,chapter_id,title,content_json,word_count,position,updated_at";

fn chapter_exists(c: &Connection, id: &str) -> Result<(), DbError> {
    let n: i64 = c.query_row("SELECT count(*) FROM chapters WHERE id=?1", [id], |r| r.get(0))?;
    if n == 0 { Err(DbError::NotFound) } else { Ok(()) }
}

pub fn scene_create(c: &Connection, chapter_id: &str, title: &str, index: Option<i64>, now: i64) -> Result<Scene, DbError> {
    let tx = super::Atomic::begin(c)?;
    chapter_exists(c, chapter_id)?;
    let id = new_id();
    let mut ids = ordered_ids(c, "scenes", "chapter_id", chapter_id)?;
    c.execute(
        "INSERT INTO scenes(id,chapter_id,title,content_json,word_count,position,updated_at) VALUES(?1,?2,?3,?4,0,-1,?5)",
        params![id, chapter_id, title, EMPTY_DOC, now],
    )?;
    insert_clamped(&mut ids, id.clone(), index);
    write_positions(c, "scenes", &ids)?;
    let s = scene_get(c, &id)?;
    tx.commit()?;
    Ok(s)
}

pub fn scene_get(c: &Connection, id: &str) -> Result<Scene, DbError> {
    c.query_row(&format!("SELECT {SCENE_COLS} FROM scenes WHERE id=?1"), [id], scene_row)
        .optional()?
        .ok_or(DbError::NotFound)
}

pub fn scene_rename(c: &Connection, id: &str, title: &str) -> Result<(), DbError> {
    let n = c.execute("UPDATE scenes SET title=?1 WHERE id=?2", params![title, id])?;
    if n == 0 { Err(DbError::NotFound) } else { Ok(()) }
}

fn add_version(c: &Connection, scene_id: &str, content_json: &str, word_count: i64, now: i64) -> Result<(), DbError> {
    c.execute(
        "INSERT INTO scene_versions(id,scene_id,content_json,word_count,created_at) VALUES(?1,?2,?3,?4,?5)",
        params![new_id(), scene_id, content_json, word_count, now],
    )?;
    Ok(())
}

pub fn scene_save(c: &Connection, id: &str, content_json: &str, word_count: i64, now: i64) -> Result<(), DbError> {
    let tx = super::Atomic::begin(c)?;
    let n = c.execute(
        "UPDATE scenes SET content_json=?1, word_count=?2, updated_at=?3 WHERE id=?4",
        params![content_json, word_count, now, id],
    )?;
    if n == 0 {
        return Err(DbError::NotFound);
    }
    let last: Option<i64> =
        c.query_row("SELECT max(created_at) FROM scene_versions WHERE scene_id=?1", [id], |r| r.get(0))?;
    if last.is_none_or(|t| now - t > VERSION_INTERVAL_MS) {
        add_version(c, id, content_json, word_count, now)?;
    }
    tx.commit()?;
    Ok(())
}

pub fn scene_move(c: &Connection, id: &str, to_chapter_id: &str, to_index: i64) -> Result<(), DbError> {
    let tx = super::Atomic::begin(c)?;
    let from = scene_get(c, id)?.chapter_id;
    chapter_exists(c, to_chapter_id)?;
    c.execute("UPDATE scenes SET chapter_id=?1 WHERE id=?2", params![to_chapter_id, id])?;
    let mut target = ordered_ids(c, "scenes", "chapter_id", to_chapter_id)?;
    target.retain(|x| x != id);
    insert_clamped(&mut target, id.to_string(), Some(to_index));
    write_positions(c, "scenes", &target)?;
    if from != to_chapter_id {
        write_positions(c, "scenes", &ordered_ids(c, "scenes", "chapter_id", &from)?)?;
    }
    tx.commit()?;
    Ok(())
}

pub fn scene_delete(c: &Connection, id: &str) -> Result<(), DbError> {
    let tx = super::Atomic::begin(c)?;
    let ch = scene_get(c, id)?.chapter_id;
    c.execute("DELETE FROM scene_versions WHERE scene_id=?1", [id])?;
    c.execute("DELETE FROM scenes WHERE id=?1", [id])?;
    write_positions(c, "scenes", &ordered_ids(c, "scenes", "chapter_id", &ch)?)?;
    tx.commit()?;
    Ok(())
}

pub fn scene_versions(c: &Connection, scene_id: &str) -> Result<Vec<Version>, DbError> {
    let mut st = c.prepare(
        "SELECT id,created_at,word_count FROM scene_versions WHERE scene_id=?1 ORDER BY created_at DESC, rowid DESC",
    )?;
    let v = st
        .query_map([scene_id], |r| Ok(Version { id: r.get(0)?, created_at: r.get(1)?, word_count: r.get(2)? }))?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

pub fn scene_restore(c: &Connection, version_id: &str, now: i64) -> Result<Scene, DbError> {
    let tx = super::Atomic::begin(c)?;
    let (scene_id, json, words): (String, String, i64) = c
        .query_row(
            "SELECT scene_id,content_json,word_count FROM scene_versions WHERE id=?1",
            [version_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or(DbError::NotFound)?;
    let cur = scene_get(c, &scene_id)?;
    add_version(c, &scene_id, &cur.content_json, cur.word_count, now)?;
    c.execute(
        "UPDATE scenes SET content_json=?1, word_count=?2, updated_at=?3 WHERE id=?4",
        params![json, words, now, scene_id],
    )?;
    let s = scene_get(c, &scene_id)?;
    tx.commit()?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::books::*;
    use crate::db::test_conn;

    fn titles(c: &Connection, chapter_id: &str) -> Vec<(String, i64)> {
        let mut st = c
            .prepare("SELECT title, position FROM scenes WHERE chapter_id=?1 ORDER BY position")
            .unwrap();
        st.query_map([chapter_id], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    fn names(c: &Connection, ch: &str) -> Vec<String> {
        titles(c, ch).into_iter().map(|t| t.0).collect()
    }

    fn contiguous(c: &Connection, ch: &str) -> bool {
        titles(c, ch).iter().enumerate().all(|(i, t)| t.1 == i as i64)
    }

    /// Livro com o capítulo inicial contendo cenas A,B,C (a cena inicial é removida).
    fn abc(c: &Connection) -> (Book, String) {
        let b = book_create(c, "L", "conselheira", 0).unwrap();
        let o = book_outline(c, &b.id).unwrap();
        let ch = o.chapters[0].chapter.id.clone();
        scene_delete(c, &o.chapters[0].scenes[0].id).unwrap();
        for t in ["A", "B", "C"] {
            scene_create(c, &ch, t, None, 0).unwrap();
        }
        (b, ch)
    }

    fn id_of(c: &Connection, ch: &str, title: &str) -> String {
        c.query_row("SELECT id FROM scenes WHERE chapter_id=?1 AND title=?2", [ch, title], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn new_book_has_one_chapter_one_scene() {
        let (_d, c) = test_conn();
        let b = book_create(&c, "Meu livro", "coautora", 5).unwrap();
        assert_eq!(b.ai_mode, "coautora");
        let o = book_outline(&c, &b.id).unwrap();
        assert_eq!(o.chapters.len(), 1);
        assert_eq!(o.chapters[0].chapter.title, "Capítulo 1");
        assert_eq!(o.chapters[0].scenes.len(), 1);
        assert_eq!(o.chapters[0].scenes[0].title, "Cena 1");
        assert_eq!(book_list(&c).unwrap(), vec![b]);
    }

    #[test]
    fn invalid_ai_mode_is_rejected() {
        let (_d, c) = test_conn();
        assert!(matches!(book_create(&c, "L", "robo", 0), Err(DbError::Invalid(_))));
    }

    #[test]
    fn scene_create_with_index_inserts_in_middle() {
        let (_d, c) = test_conn();
        let (_, ch) = abc(&c);
        scene_create(&c, &ch, "X", Some(1), 0).unwrap();
        assert_eq!(names(&c, &ch), ["A", "X", "B", "C"]);
        assert!(contiguous(&c, &ch));
    }

    #[test]
    fn move_within_chapter() {
        let (_d, c) = test_conn();
        let (_, ch) = abc(&c);
        scene_move(&c, &id_of(&c, &ch, "C"), &ch, 0).unwrap();
        assert_eq!(names(&c, &ch), ["C", "A", "B"]);
        assert!(contiguous(&c, &ch));
    }

    #[test]
    fn move_forward_within_chapter() {
        let (_d, c) = test_conn();
        let (_, ch) = abc(&c);
        scene_move(&c, &id_of(&c, &ch, "A"), &ch, 2).unwrap();
        assert_eq!(names(&c, &ch), ["B", "C", "A"]);
    }

    #[test]
    fn move_to_other_chapter_keeps_both_contiguous() {
        let (_d, c) = test_conn();
        let (b, ch1) = abc(&c);
        let ch2 = chapter_create(&c, &b.id, "Dois", None).unwrap().id;
        scene_create(&c, &ch2, "D", None, 0).unwrap();
        scene_move(&c, &id_of(&c, &ch1, "B"), &ch2, 1).unwrap();
        assert_eq!(names(&c, &ch1), ["A", "C"]);
        assert_eq!(names(&c, &ch2), ["D", "B"]);
        assert!(contiguous(&c, &ch1) && contiguous(&c, &ch2));
    }

    #[test]
    fn move_to_empty_chapter() {
        let (_d, c) = test_conn();
        let (b, ch1) = abc(&c);
        let ch2 = chapter_create(&c, &b.id, "Vazio", None).unwrap().id;
        scene_move(&c, &id_of(&c, &ch1, "A"), &ch2, 0).unwrap();
        assert_eq!(titles(&c, &ch2), [("A".to_string(), 0)]);
        assert!(contiguous(&c, &ch1));
    }

    #[test]
    fn move_index_past_end_clamps_to_end() {
        let (_d, c) = test_conn();
        let (_, ch) = abc(&c);
        scene_move(&c, &id_of(&c, &ch, "A"), &ch, 99).unwrap();
        assert_eq!(names(&c, &ch), ["B", "C", "A"]);
        assert!(contiguous(&c, &ch));
    }

    #[test]
    fn delete_nonempty_chapter_fails() {
        let (_d, c) = test_conn();
        let (b, ch) = abc(&c);
        assert!(matches!(chapter_delete(&c, &ch), Err(DbError::ChapterNotEmpty)));
        assert_eq!(names(&c, &ch), ["A", "B", "C"]);
        let empty = chapter_create(&c, &b.id, "E", None).unwrap().id;
        chapter_delete(&c, &empty).unwrap();
        assert_eq!(book_outline(&c, &b.id).unwrap().chapters.len(), 1);
    }

    #[test]
    fn chapters_reorder() {
        let (_d, c) = test_conn();
        let b = book_create(&c, "L", "conselheira", 0).unwrap();
        let two = chapter_create(&c, &b.id, "Dois", None).unwrap().id;
        chapter_create(&c, &b.id, "Meio", Some(1)).unwrap();
        chapter_move(&c, &two, 0).unwrap();
        let o = book_outline(&c, &b.id).unwrap();
        let t: Vec<_> = o.chapters.iter().map(|x| (x.chapter.title.as_str(), x.chapter.position)).collect();
        assert_eq!(t, [("Dois", 0), ("Capítulo 1", 1), ("Meio", 2)]);
    }

    #[test]
    fn save_creates_version_only_after_10min() {
        let (_d, c) = test_conn();
        let (_, ch) = abc(&c);
        let id = id_of(&c, &ch, "A");
        scene_save(&c, &id, "{\"v\":1}", 1, 1_000).unwrap();
        scene_save(&c, &id, "{\"v\":2}", 2, 2_000).unwrap();
        assert_eq!(scene_versions(&c, &id).unwrap().len(), 1);
        scene_save(&c, &id, "{\"v\":3}", 3, 2_000 + 11 * 60 * 1000).unwrap();
        assert_eq!(scene_versions(&c, &id).unwrap().len(), 2);
        let s = scene_get(&c, &id).unwrap();
        assert_eq!((s.content_json.as_str(), s.word_count), ("{\"v\":3}", 3));
    }

    #[test]
    fn restore_saves_current_first() {
        let (_d, c) = test_conn();
        let (_, ch) = abc(&c);
        let id = id_of(&c, &ch, "A");
        scene_save(&c, &id, "{\"v\":\"antigo\"}", 1, 1_000).unwrap();
        let old = scene_versions(&c, &id).unwrap()[0].id.clone();
        scene_save(&c, &id, "{\"v\":\"novo\"}", 1, 2_000).unwrap();
        let s = scene_restore(&c, &old, 3_000).unwrap();
        assert_eq!(s.content_json, "{\"v\":\"antigo\"}");
        let vs = scene_versions(&c, &id).unwrap();
        assert_eq!(vs.len(), 2);
        let newest: String = c
            .query_row("SELECT content_json FROM scene_versions WHERE id=?1", [&vs[0].id], |r| r.get(0))
            .unwrap();
        assert_eq!(newest, "{\"v\":\"novo\"}");
    }
}
