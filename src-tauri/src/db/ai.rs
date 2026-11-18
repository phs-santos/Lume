//! Armazenamento da IA: Bíblia viva, resumos de cena, conversas, propostas e gasto.

use super::{new_id, DbError};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub const KINDS: [&str; 4] = ["personagem", "lugar", "fato", "linha_do_tempo"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiNote {
    pub fact: String,
    pub source_scene_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BibleEntry {
    pub id: String,
    pub book_id: String,
    pub kind: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub body: String,
    pub ai_notes: Vec<AiNote>,
    pub story_order: Option<i64>,
    pub source_scene_id: Option<String>,
    pub edited_by_user: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Conversation {
    pub id: String,
    pub book_id: String,
    pub title: String,
    pub rolling_summary: String,
    /// Quantas mensagens (as mais antigas) já estão dentro de `rolling_summary`.
    pub folded_count: i64,
    pub closed: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    /// `user`, `assistant` ou `note` (linha discreta: "consultei a cena X").
    pub role: String,
    pub content: String,
    pub tool_name: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Proposal {
    pub id: String,
    pub book_id: String,
    pub conversation_id: Option<String>,
    pub scene_id: Option<String>,
    pub text: String,
    pub status: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UsageRow {
    pub month: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: Option<f64>,
}

/// Minúsculas e sem acentos, para comparar nomes ("RAFAÉL" == "rafael").
pub fn fold(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .map(|ch| match ch {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

fn check_kind(kind: &str) -> Result<(), DbError> {
    if KINDS.contains(&kind) { Ok(()) } else { Err(DbError::Invalid(format!("tipo de ficha: {kind}"))) }
}

const ENTRY_COLS: &str = "id,book_id,kind,name,aliases_json,body,ai_notes_json,story_order,source_scene_id,edited_by_user,updated_at";

fn entry_row(r: &rusqlite::Row) -> rusqlite::Result<BibleEntry> {
    let aliases: String = r.get(4)?;
    let notes: String = r.get(6)?;
    Ok(BibleEntry {
        id: r.get(0)?,
        book_id: r.get(1)?,
        kind: r.get(2)?,
        name: r.get(3)?,
        aliases: serde_json::from_str(&aliases).unwrap_or_default(),
        body: r.get(5)?,
        ai_notes: serde_json::from_str(&notes).unwrap_or_default(),
        story_order: r.get(7)?,
        source_scene_id: r.get(8)?,
        edited_by_user: r.get::<_, i64>(9)? != 0,
        updated_at: r.get(10)?,
    })
}

pub fn bible_list(c: &Connection, book_id: &str) -> Result<Vec<BibleEntry>, DbError> {
    let mut st = c.prepare(&format!(
        "SELECT {ENTRY_COLS} FROM bible_entries WHERE book_id=?1 ORDER BY kind, story_order IS NULL, story_order, name COLLATE NOCASE"
    ))?;
    let v = st.query_map([book_id], entry_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn bible_get(c: &Connection, id: &str) -> Result<BibleEntry, DbError> {
    c.query_row(&format!("SELECT {ENTRY_COLS} FROM bible_entries WHERE id=?1"), [id], entry_row)
        .optional()?
        .ok_or(DbError::NotFound)
}

/// Acha uma ficha pelo nome ou por um apelido, sem diferenciar maiúsculas e acentos.
pub fn bible_find(c: &Connection, book_id: &str, kind: &str, name_or_alias: &str) -> Result<Option<BibleEntry>, DbError> {
    let key = fold(name_or_alias);
    Ok(bible_list(c, book_id)?
        .into_iter()
        .find(|e| e.kind == kind && (fold(&e.name) == key || e.aliases.iter().any(|a| fold(a) == key))))
}

fn merge_aliases(e: &BibleEntry, name: &str, aliases: &[String]) -> Vec<String> {
    let mut out = e.aliases.clone();
    for a in std::iter::once(name.to_string()).chain(aliases.iter().cloned()) {
        let a = a.trim().to_string();
        if a.is_empty() || fold(&a) == fold(&e.name) || out.iter().any(|x| fold(x) == fold(&a)) {
            continue;
        }
        out.push(a);
    }
    out
}

/// Criação/atualização feita pela IA. Ficha editada à mão: a IA só acrescenta uma nota.
#[allow(clippy::too_many_arguments)]
pub fn bible_upsert_ai(
    c: &Connection,
    book_id: &str,
    kind: &str,
    name: &str,
    aliases: &[String],
    fact: &str,
    source_scene_id: Option<&str>,
    story_order: Option<i64>,
    now: i64,
) -> Result<BibleEntry, DbError> {
    check_kind(kind)?;
    let name = name.trim();
    if name.is_empty() || fact.trim().is_empty() {
        return Err(DbError::Invalid("ficha sem nome ou sem fato".into()));
    }
    let found = std::iter::once(name)
        .chain(aliases.iter().map(|s| s.as_str()))
        .find_map(|n| bible_find(c, book_id, kind, n).transpose())
        .transpose()?;
    match found {
        None => {
            let id = new_id();
            let al: Vec<String> = aliases.iter().map(|a| a.trim().to_string()).filter(|a| !a.is_empty() && fold(a) != fold(name)).collect();
            c.execute(
                "INSERT INTO bible_entries(id,book_id,kind,name,aliases_json,body,ai_notes_json,story_order,source_scene_id,edited_by_user,updated_at)
                 VALUES(?1,?2,?3,?4,?5,?6,'[]',?7,?8,0,?9)",
                params![id, book_id, kind, name, serde_json::to_string(&al).unwrap(), fact.trim(), story_order, source_scene_id, now],
            )?;
            bible_get(c, &id)
        }
        Some(e) => {
            let al = serde_json::to_string(&merge_aliases(&e, name, aliases)).unwrap();
            let order = story_order.or(e.story_order);
            if e.edited_by_user {
                let mut notes = e.ai_notes.clone();
                if !notes.iter().any(|n| fold(&n.fact) == fold(fact)) && fold(&e.body) != fold(fact) {
                    notes.push(AiNote { fact: fact.trim().to_string(), source_scene_id: source_scene_id.map(String::from) });
                }
                c.execute(
                    "UPDATE bible_entries SET aliases_json=?2, ai_notes_json=?3, updated_at=?4 WHERE id=?1",
                    params![e.id, al, serde_json::to_string(&notes).unwrap(), now],
                )?;
            } else {
                c.execute(
                    "UPDATE bible_entries SET aliases_json=?2, body=?3, story_order=?4, source_scene_id=COALESCE(?5, source_scene_id), updated_at=?6 WHERE id=?1",
                    params![e.id, al, fact.trim(), order, source_scene_id, now],
                )?;
            }
            bible_get(c, &e.id)
        }
    }
}

/// Edição pela pessoa. A partir daqui a IA não reescreve mais o texto da ficha.
pub fn bible_update_user(c: &Connection, id: &str, name: &str, aliases: &[String], body: &str, story_order: Option<i64>, now: i64) -> Result<BibleEntry, DbError> {
    if name.trim().is_empty() {
        return Err(DbError::Invalid("ficha sem nome".into()));
    }
    let al: Vec<String> = aliases.iter().map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
    let n = c.execute(
        "UPDATE bible_entries SET name=?2, aliases_json=?3, body=?4, story_order=?5, edited_by_user=1, updated_at=?6 WHERE id=?1",
        params![id, name.trim(), serde_json::to_string(&al).unwrap(), body, story_order, now],
    )?;
    if n == 0 { return Err(DbError::NotFound); }
    bible_get(c, id)
}

/// Junta uma nota da IA ao texto da ficha (botão "incorporar").
pub fn bible_absorb_note(c: &Connection, id: &str, index: usize, now: i64) -> Result<BibleEntry, DbError> {
    let mut e = bible_get(c, id)?;
    if index >= e.ai_notes.len() { return Err(DbError::NotFound); }
    let note = e.ai_notes.remove(index);
    let body = if e.body.trim().is_empty() { note.fact } else { format!("{}\n{}", e.body.trim_end(), note.fact) };
    c.execute(
        "UPDATE bible_entries SET body=?2, ai_notes_json=?3, updated_at=?4 WHERE id=?1",
        params![id, body, serde_json::to_string(&e.ai_notes).unwrap(), now],
    )?;
    bible_get(c, id)
}

pub fn bible_delete(c: &Connection, id: &str) -> Result<(), DbError> {
    c.execute("DELETE FROM bible_entries WHERE id=?1", [id])?;
    Ok(())
}

pub fn digest_get(c: &Connection, scene_id: &str) -> Result<Option<(String, String)>, DbError> {
    Ok(c.query_row("SELECT content_hash, summary FROM ai_scene_digest WHERE scene_id=?1", [scene_id], |r| Ok((r.get(0)?, r.get(1)?))).optional()?)
}

pub fn digest_put(c: &Connection, scene_id: &str, hash: &str, summary: &str, now: i64) -> Result<(), DbError> {
    c.execute(
        "INSERT INTO ai_scene_digest(scene_id,content_hash,summary,updated_at) VALUES(?1,?2,?3,?4)
         ON CONFLICT(scene_id) DO UPDATE SET content_hash=excluded.content_hash, summary=excluded.summary, updated_at=excluded.updated_at",
        params![scene_id, hash, summary, now],
    )?;
    Ok(())
}

const CONV_COLS: &str = "id,book_id,title,rolling_summary,folded_count,closed,created_at,updated_at";

fn conv_row(r: &rusqlite::Row) -> rusqlite::Result<Conversation> {
    Ok(Conversation {
        id: r.get(0)?,
        book_id: r.get(1)?,
        title: r.get(2)?,
        rolling_summary: r.get(3)?,
        folded_count: r.get(4)?,
        closed: r.get::<_, i64>(5)? != 0,
        created_at: r.get(6)?,
        updated_at: r.get(7)?,
    })
}

pub fn conversation_get(c: &Connection, id: &str) -> Result<Conversation, DbError> {
    c.query_row(&format!("SELECT {CONV_COLS} FROM ai_conversations WHERE id=?1"), [id], conv_row).optional()?.ok_or(DbError::NotFound)
}

fn create_conversation(c: &Connection, book_id: &str, now: i64) -> Result<Conversation, DbError> {
    let id = new_id();
    c.execute(
        "INSERT INTO ai_conversations(id,book_id,title,created_at,updated_at) VALUES(?1,?2,'Conversa',?3,?3)",
        params![id, book_id, now],
    )?;
    conversation_get(c, &id)
}

/// A conversa aberta do livro; cria uma se não houver.
pub fn conversation_current(c: &Connection, book_id: &str, now: i64) -> Result<Conversation, DbError> {
    let open = c
        .query_row(
            &format!("SELECT {CONV_COLS} FROM ai_conversations WHERE book_id=?1 AND closed=0 ORDER BY updated_at DESC LIMIT 1"),
            [book_id],
            conv_row,
        )
        .optional()?;
    match open {
        Some(cv) => Ok(cv),
        None => create_conversation(c, book_id, now),
    }
}

/// Fecha a conversa atual e começa outra.
pub fn conversation_new(c: &Connection, book_id: &str, now: i64) -> Result<Conversation, DbError> {
    c.execute("UPDATE ai_conversations SET closed=1 WHERE book_id=?1", [book_id])?;
    create_conversation(c, book_id, now)
}

pub fn conversation_list(c: &Connection, book_id: &str) -> Result<Vec<Conversation>, DbError> {
    let mut st = c.prepare(&format!("SELECT {CONV_COLS} FROM ai_conversations WHERE book_id=?1 ORDER BY created_at DESC"))?;
    let v = st.query_map([book_id], conv_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

/// Reabre uma conversa antiga (e fecha a que estava aberta).
pub fn conversation_open(c: &Connection, id: &str) -> Result<Conversation, DbError> {
    let cv = conversation_get(c, id)?;
    c.execute("UPDATE ai_conversations SET closed=1 WHERE book_id=?1", [&cv.book_id])?;
    c.execute("UPDATE ai_conversations SET closed=0 WHERE id=?1", [id])?;
    conversation_get(c, id)
}

pub fn conversation_set_title(c: &Connection, id: &str, title: &str) -> Result<(), DbError> {
    c.execute("UPDATE ai_conversations SET title=?2 WHERE id=?1", params![id, title])?;
    Ok(())
}

/// Apaga a conversa e as mensagens; propostas dela ficam sem conversa.
pub fn conversation_delete(c: &Connection, id: &str) -> Result<(), DbError> {
    if c.execute("DELETE FROM ai_conversations WHERE id=?1", [id])? == 0 {
        return Err(DbError::NotFound);
    }
    Ok(())
}

pub fn conversation_set_summary(c: &Connection, id: &str, summary: &str, folded_count: i64) -> Result<(), DbError> {
    c.execute("UPDATE ai_conversations SET rolling_summary=?2, folded_count=?3 WHERE id=?1", params![id, summary, folded_count])?;
    Ok(())
}

pub fn message_add(c: &Connection, conversation_id: &str, role: &str, content: &str, tool_name: Option<&str>, now: i64) -> Result<Message, DbError> {
    let id = new_id();
    let seq: i64 = c.query_row("SELECT COALESCE(MAX(seq),0)+1 FROM ai_messages WHERE conversation_id=?1", [conversation_id], |r| r.get(0))?;
    c.execute(
        "INSERT INTO ai_messages(id,conversation_id,role,content,tool_name,created_at,seq) VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![id, conversation_id, role, content, tool_name, now, seq],
    )?;
    c.execute("UPDATE ai_conversations SET updated_at=?2 WHERE id=?1", params![conversation_id, now])?;
    Ok(Message { id, conversation_id: conversation_id.into(), role: role.into(), content: content.into(), tool_name: tool_name.map(String::from), created_at: now })
}

pub fn messages_all(c: &Connection, conversation_id: &str) -> Result<Vec<Message>, DbError> {
    let mut st = c.prepare("SELECT id,conversation_id,role,content,tool_name,created_at FROM ai_messages WHERE conversation_id=?1 ORDER BY seq")?;
    let v = st
        .query_map([conversation_id], |r| {
            Ok(Message { id: r.get(0)?, conversation_id: r.get(1)?, role: r.get(2)?, content: r.get(3)?, tool_name: r.get(4)?, created_at: r.get(5)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn message_delete(c: &Connection, id: &str) -> Result<(), DbError> {
    c.execute("DELETE FROM ai_messages WHERE id=?1", [id])?;
    Ok(())
}

const PROP_COLS: &str = "id,book_id,conversation_id,scene_id,text,status,created_at";

fn prop_row(r: &rusqlite::Row) -> rusqlite::Result<Proposal> {
    Ok(Proposal { id: r.get(0)?, book_id: r.get(1)?, conversation_id: r.get(2)?, scene_id: r.get(3)?, text: r.get(4)?, status: r.get(5)?, created_at: r.get(6)? })
}

pub fn proposal_add(c: &Connection, book_id: &str, conversation_id: Option<&str>, scene_id: Option<&str>, text: &str, now: i64) -> Result<Proposal, DbError> {
    let id = new_id();
    c.execute(
        &format!("INSERT INTO ai_proposals({PROP_COLS}) VALUES(?1,?2,?3,?4,?5,'pendente',?6)"),
        params![id, book_id, conversation_id, scene_id, text, now],
    )?;
    Ok(c.query_row(&format!("SELECT {PROP_COLS} FROM ai_proposals WHERE id=?1"), [&id], prop_row)?)
}

pub fn proposal_set_status(c: &Connection, id: &str, status: &str) -> Result<(), DbError> {
    if !["pendente", "inserida", "virou_cena", "descartada"].contains(&status) {
        return Err(DbError::Invalid(format!("status: {status}")));
    }
    c.execute("UPDATE ai_proposals SET status=?2 WHERE id=?1", params![id, status])?;
    Ok(())
}

pub fn proposals_pending(c: &Connection, book_id: &str) -> Result<Vec<Proposal>, DbError> {
    let mut st = c.prepare(&format!("SELECT {PROP_COLS} FROM ai_proposals WHERE book_id=?1 AND status='pendente' ORDER BY created_at"))?;
    let v = st.query_map([book_id], prop_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn usage_add(c: &Connection, month: &str, model: &str, input_tokens: i64, output_tokens: i64, cost: Option<f64>) -> Result<(), DbError> {
    c.execute(
        "INSERT INTO ai_usage(month,model,input_tokens,output_tokens,cost) VALUES(?1,?2,?3,?4,?5)
         ON CONFLICT(month,model) DO UPDATE SET input_tokens=input_tokens+excluded.input_tokens,
           output_tokens=output_tokens+excluded.output_tokens,
           cost=CASE WHEN excluded.cost IS NULL THEN cost ELSE COALESCE(cost,0)+excluded.cost END",
        params![month, model, input_tokens, output_tokens, cost],
    )?;
    Ok(())
}

/// Gasto estimado do mês, em US$.
pub fn usage_cost_month(c: &Connection, month: &str) -> Result<f64, DbError> {
    Ok(c.query_row("SELECT COALESCE(SUM(cost),0) FROM ai_usage WHERE month=?1", [month], |r| r.get(0))?)
}

pub fn usage_month(c: &Connection, month: &str) -> Result<Vec<UsageRow>, DbError> {
    let mut st = c.prepare("SELECT month,model,input_tokens,output_tokens,cost FROM ai_usage WHERE month=?1 ORDER BY model")?;
    let v = st
        .query_map([month], |r| Ok(UsageRow { month: r.get(0)?, model: r.get(1)?, input_tokens: r.get(2)?, output_tokens: r.get(3)?, cost: r.get(4)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{books, scenes, test_conn};
    use rusqlite::Connection;

    fn book(c: &Connection) -> (String, String) {
        let b = books::book_create(c, "L", "conselheira", 0).unwrap();
        let ch = books::book_outline(c, &b.id).unwrap().chapters[0].chapter.id.clone();
        let s = scenes::scene_create(c, &ch, "Cena", None, 0).unwrap();
        (b.id, s.id)
    }

    #[test]
    fn migration_adds_ai_tables() {
        let (_d, c) = test_conn();
        let v: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 3);
        c.query_row("SELECT count(*) FROM bible_entries", [], |r| r.get::<_, i64>(0)).unwrap();
    }

    #[test]
    fn find_matches_alias_ignoring_case_and_accents() {
        let (_d, c) = test_conn();
        let (b, s) = book(&c);
        bible_upsert_ai(&c, &b, "personagem", "Rafael", &["Rafa".into()], "Programador", Some(&s), None, 1).unwrap();
        assert!(bible_find(&c, &b, "personagem", "rafa").unwrap().is_some());
        assert!(bible_find(&c, &b, "personagem", "RAFAÉL").unwrap().is_some());
        assert!(bible_find(&c, &b, "lugar", "rafa").unwrap().is_none());
    }

    #[test]
    fn upsert_creates_then_updates_same_entry() {
        let (_d, c) = test_conn();
        let (b, s) = book(&c);
        bible_upsert_ai(&c, &b, "personagem", "Rafa", &[], "Programador", Some(&s), None, 1).unwrap();
        let e = bible_upsert_ai(&c, &b, "personagem", "Rafael", &["Rafa".into()], "Programador júnior", Some(&s), None, 2).unwrap();
        let all = bible_list(&c, &b).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(e.body, "Programador júnior");
        assert!(e.aliases.iter().any(|a| a == "Rafael") || e.name == "Rafael");
    }

    #[test]
    fn upsert_on_user_edited_only_appends_note() {
        let (_d, c) = test_conn();
        let (b, s) = book(&c);
        let e = bible_upsert_ai(&c, &b, "personagem", "Rafa", &[], "Programador", Some(&s), None, 1).unwrap();
        bible_update_user(&c, &e.id, "Rafa", &[], "Meu texto", None, 2).unwrap();
        bible_upsert_ai(&c, &b, "personagem", "Rafa", &[], "Mora em SP", Some(&s), None, 3).unwrap();
        let e = bible_upsert_ai(&c, &b, "personagem", "Rafa", &[], "Mora em SP", Some(&s), None, 4).unwrap();
        assert_eq!(e.body, "Meu texto");
        assert!(e.edited_by_user);
        assert_eq!(e.ai_notes.len(), 1);
        assert_eq!(e.ai_notes[0].fact, "Mora em SP");
    }

    #[test]
    fn timeline_sorted_by_story_order() {
        let (_d, c) = test_conn();
        let (b, _) = book(&c);
        bible_upsert_ai(&c, &b, "linha_do_tempo", "Depois", &[], "x", None, Some(2), 1).unwrap();
        bible_upsert_ai(&c, &b, "linha_do_tempo", "Antes", &[], "x", None, Some(1), 1).unwrap();
        let names: Vec<_> = bible_list(&c, &b).unwrap().into_iter().map(|e| e.name).collect();
        assert_eq!(names, vec!["Antes", "Depois"]);
    }

    #[test]
    fn deleting_scene_nulls_source() {
        let (_d, c) = test_conn();
        let (b, s) = book(&c);
        bible_upsert_ai(&c, &b, "lugar", "Bar", &[], "x", Some(&s), None, 1).unwrap();
        digest_put(&c, &s, "h", "r", 1).unwrap();
        scenes::scene_delete(&c, &s).unwrap();
        assert_eq!(bible_list(&c, &b).unwrap()[0].source_scene_id, None);
        assert_eq!(digest_get(&c, &s).unwrap(), None);
    }

    #[test]
    fn conversation_delete_removes_it_and_its_messages() {
        let (_d, c) = test_conn();
        let (b, _) = book(&c);
        let cv = conversation_current(&c, &b, 1).unwrap();
        message_add(&c, &cv.id, "user", "oi", None, 1).unwrap();
        conversation_delete(&c, &cv.id).unwrap();
        assert!(conversation_list(&c, &b).unwrap().is_empty());
        let n: i64 = c.query_row("SELECT COUNT(*) FROM ai_messages", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
        assert!(matches!(conversation_delete(&c, &cv.id), Err(DbError::NotFound)));
    }

    #[test]
    fn conversation_new_closes_current_and_list_returns_both() {
        let (_d, c) = test_conn();
        let (b, _) = book(&c);
        let a = conversation_current(&c, &b, 1).unwrap();
        assert_eq!(conversation_current(&c, &b, 2).unwrap().id, a.id);
        let n = conversation_new(&c, &b, 3).unwrap();
        assert_ne!(n.id, a.id);
        assert_eq!(conversation_current(&c, &b, 4).unwrap().id, n.id);
        assert_eq!(conversation_list(&c, &b).unwrap().len(), 2);
        conversation_open(&c, &a.id).unwrap();
        assert_eq!(conversation_current(&c, &b, 5).unwrap().id, a.id);
    }

    #[test]
    fn messages_keep_order_and_delete() {
        let (_d, c) = test_conn();
        let (b, _) = book(&c);
        let cv = conversation_current(&c, &b, 1).unwrap();
        let m1 = message_add(&c, &cv.id, "user", "oi", None, 5).unwrap();
        message_add(&c, &cv.id, "assistant", "olá", None, 5).unwrap();
        let all = messages_all(&c, &cv.id).unwrap();
        assert_eq!(all.iter().map(|m| m.content.as_str()).collect::<Vec<_>>(), vec!["oi", "olá"]);
        message_delete(&c, &m1.id).unwrap();
        assert_eq!(messages_all(&c, &cv.id).unwrap().len(), 1);
    }

    #[test]
    fn digest_put_overwrites() {
        let (_d, c) = test_conn();
        let (_, s) = book(&c);
        digest_put(&c, &s, "h1", "a", 1).unwrap();
        digest_put(&c, &s, "h2", "b", 2).unwrap();
        assert_eq!(digest_get(&c, &s).unwrap(), Some(("h2".into(), "b".into())));
    }

    #[test]
    fn proposals_and_usage() {
        let (_d, c) = test_conn();
        let (b, s) = book(&c);
        let p = proposal_add(&c, &b, None, Some(&s), "Texto", 1).unwrap();
        assert_eq!(proposals_pending(&c, &b).unwrap().len(), 1);
        proposal_set_status(&c, &p.id, "descartada").unwrap();
        assert!(proposals_pending(&c, &b).unwrap().is_empty());
        usage_add(&c, "2026-11", "m", 10, 5, Some(0.01)).unwrap();
        usage_add(&c, "2026-11", "m", 1, 1, None).unwrap();
        let u = usage_month(&c, "2026-11").unwrap();
        assert_eq!((u[0].input_tokens, u[0].output_tokens), (11, 6));
    }
}
