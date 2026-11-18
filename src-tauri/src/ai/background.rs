//! Agente de fundo: resumo curto de cada cena e atualização da Bíblia viva.

use crate::ai::chat::{record_usage, AiConfig};
use crate::ai::context::bible_compact;
use crate::ai::provider::{ChatModel, Msg};
use crate::ai::text::{content_hash, plain_text};
use crate::db::{ai, books, scenes, DbError};
use crate::state::SharedDb;
use rusqlite::Connection;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Deserialize)]
struct Card {
    tipo: String,
    nome: String,
    #[serde(default)]
    apelidos: Vec<String>,
    fato: String,
    #[serde(default)]
    ordem: Option<i64>,
}

#[derive(Deserialize)]
struct Digest {
    resumo: String,
    #[serde(default)]
    fichas: Vec<Card>,
}

const EMPTY_SUMMARY: &str = "(cena vazia)";

fn with<T>(db: &SharedDb, f: impl FnOnce(&Connection) -> Result<T, DbError>) -> Result<T, String> {
    let g = db.lock().unwrap();
    let c = g.as_ref().ok_or_else(|| "LOCKED".to_string())?;
    f(c).map_err(|e| e.to_string())
}

/// Tira cercas ```json e texto em volta, ficando com o primeiro objeto JSON.
fn extract_json(s: &str) -> Option<&str> {
    let a = s.find('{')?;
    let b = s.rfind('}')?;
    (b > a).then(|| &s[a..=b])
}

fn book_of_scene(c: &Connection, scene_id: &str) -> Result<String, DbError> {
    let ch = scenes::scene_get(c, scene_id)?.chapter_id;
    Ok(c.query_row("SELECT book_id FROM chapters WHERE id=?1", [ch], |r| r.get(0))?)
}

/// Resume a cena e atualiza a Bíblia. `Ok(false)` quando o texto não mudou desde a última vez.
pub fn digest_scene(db: &SharedDb, model: &dyn ChatModel, scene_id: &str, now: &dyn Fn() -> i64) -> Result<bool, String> {
    let (cfg, book_id, title, text, hash, bible, prev) = with(db, |c| {
        let s = scenes::scene_get(c, scene_id)?;
        let book_id = book_of_scene(c, scene_id)?;
        let hash = content_hash(&s.content_json);
        Ok((AiConfig::load(c)?, book_id.clone(), s.title, plain_text(&s.content_json), hash, bible_compact(c, &book_id)?, ai::digest_get(c, scene_id)?))
    })?;
    if prev.as_ref().map(|p| p.0.as_str()) == Some(hash.as_str()) {
        return Ok(false);
    }
    if text.trim().is_empty() {
        with(db, |c| ai::digest_put(c, scene_id, &hash, EMPTY_SUMMARY, now()))?;
        return Ok(true);
    }
    // limite do mês atingido: o resumo espera (a cena continua pendente)
    if let Some(m) = with(db, |c| crate::ai::chat::limit_reached(c, now()))? {
        return Err(m);
    }
    let prompt = format!(
        "Você mantém a Bíblia de um livro. Leia a cena e responda SÓ com JSON, sem comentários:\n\
{{\"resumo\": \"2 a 3 frases sobre o que acontece\", \"fichas\": [{{\"tipo\": \"personagem|lugar|fato|linha_do_tempo\", \"nome\": \"...\", \"apelidos\": [\"...\"], \"fato\": \"descrição curta e estável\", \"ordem\": 0}}]}}\n\
Regras: inclua só fichas novas ou que mudaram com esta cena; use o mesmo nome das fichas existentes quando for a mesma coisa; \
'ordem' só em linha_do_tempo (posição na história); não invente nada que não esteja no texto.\n\n\
# Fichas existentes\n{bible}\n\n# Cena: {title}\n{text}"
    );
    let turn = model.complete(&cfg.background_model, &[Msg::user(&prompt)], &[], &mut |_| {}).map_err(|e| e.to_string())?;
    record_usage(db, &cfg.background_model, &turn.usage, now());
    let parsed: Digest = extract_json(&turn.text)
        .and_then(|j| serde_json::from_str(j).ok())
        .ok_or_else(|| "resposta do agente de fundo não é JSON válido".to_string())?;
    with(db, |c| {
        let tx = crate::db::Atomic::begin(c)?;
        for card in &parsed.fichas {
            match ai::bible_upsert_ai(c, &book_id, &card.tipo, &card.nome, &card.apelidos, &card.fato, Some(scene_id), card.ordem, now()) {
                Ok(_) | Err(DbError::Invalid(_)) => {} // ficha inválida é ignorada, o resto segue
                Err(e) => return Err(e),
            }
        }
        ai::digest_put(c, scene_id, &hash, parsed.resumo.trim(), now())?;
        tx.commit()
    })?;
    Ok(true)
}

/// Cenas sem resumo ou com texto mudado, na ordem do livro.
pub fn pending_scenes(c: &Connection, book_id: &str) -> Result<Vec<String>, DbError> {
    let mut out = vec![];
    for ch in books::book_outline(c, book_id)?.chapters {
        for sc in ch.scenes {
            let s = scenes::scene_get(c, &sc.id)?;
            let h = content_hash(&s.content_json);
            if ai::digest_get(c, &sc.id)?.map(|d| d.0) != Some(h) {
                out.push(sc.id);
            }
        }
    }
    Ok(out)
}

/// Processa as pendentes uma a uma. Falha numa cena não para a fila: ela fica pendente para depois.
pub fn run_queue(db: &SharedDb, model: &dyn ChatModel, book_id: &str, cancel: &AtomicBool, progress: &mut dyn FnMut(usize, usize), now: &dyn Fn() -> i64) -> Result<(), String> {
    let pending = with(db, |c| pending_scenes(c, book_id))?;
    let total = pending.len();
    progress(0, total);
    for (i, id) in pending.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            return Ok(());
        }
        let _ = digest_scene(db, model, id, now);
        progress(i + 1, total);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::testkit::{self, shared, text_turn, FakeModel};
    use crate::db::{ai, scenes, test_conn};
    use std::sync::atomic::AtomicBool;

    fn now() -> i64 { 1_794_900_000_000 }

    #[test]
    fn monthly_limit_pauses_digest() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        crate::db::settings::setting_set(&c, "ai.monthly_limit", "1").unwrap();
        ai::usage_add(&c, &crate::ai::chat::month_of(now()), "x", 0, 0, Some(2.0)).unwrap();
        let db = shared(c);
        let m = FakeModel::new(vec![text_turn("{}")]);
        assert!(digest_scene(&db, &m, &b.scenes[0], &now).is_err());
        assert!(m.calls.lock().unwrap().is_empty());
    }
    const GOOD: &str = r#"```json
{"resumo":"Rafa programa de madrugada.","fichas":[{"tipo":"personagem","nome":"Rafa","apelidos":["Rafael"],"fato":"Programador insone"}]}
```"#;

    #[test]
    fn applies_cards_and_summary() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let db = shared(c);
        let m = FakeModel::new(vec![text_turn(GOOD)]);
        assert!(digest_scene(&db, &m, &b.scenes[0], &now).unwrap());
        let g = db.lock().unwrap();
        let c = g.as_ref().unwrap();
        assert_eq!(ai::digest_get(c, &b.scenes[0]).unwrap().unwrap().1, "Rafa programa de madrugada.");
        let e = &ai::bible_list(c, &b.id).unwrap()[0];
        assert_eq!((e.name.as_str(), e.source_scene_id.as_deref()), ("Rafa", Some(b.scenes[0].as_str())));
        assert_eq!(m.calls.lock().unwrap()[0].0, crate::ai::chat::DEFAULT_BACKGROUND_MODEL);
    }

    #[test]
    fn unchanged_scene_skips_model() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let db = shared(c);
        let m = FakeModel::new(vec![text_turn(GOOD), text_turn(GOOD)]);
        assert!(digest_scene(&db, &m, &b.scenes[0], &now).unwrap());
        assert!(!digest_scene(&db, &m, &b.scenes[0], &now).unwrap());
        assert_eq!(m.n_calls(), 1);
    }

    #[test]
    fn empty_scene_needs_no_model() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let s = scenes::scene_create(&c, &b.chapter, "Vazia", None, 1).unwrap();
        let db = shared(c);
        let m = FakeModel::new(vec![]);
        digest_scene(&db, &m, &s.id, &now).unwrap();
        assert_eq!(m.n_calls(), 0);
        assert!(pending_scenes(db.lock().unwrap().as_ref().unwrap(), &b.id).unwrap().iter().all(|id| *id != s.id));
    }

    #[test]
    fn invalid_json_changes_nothing() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let db = shared(c);
        let m = FakeModel::new(vec![text_turn("não sei responder em json")]);
        assert!(digest_scene(&db, &m, &b.scenes[0], &now).is_err());
        let g = db.lock().unwrap();
        let c = g.as_ref().unwrap();
        assert!(ai::digest_get(c, &b.scenes[0]).unwrap().is_none());
        assert!(ai::bible_list(c, &b.id).unwrap().is_empty());
    }

    #[test]
    fn pending_lists_new_and_changed_only() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let db = shared(c);
        let m = FakeModel::new(vec![text_turn(GOOD), text_turn(GOOD)]);
        digest_scene(&db, &m, &b.scenes[0], &now).unwrap();
        digest_scene(&db, &m, &b.scenes[1], &now).unwrap();
        let g = db.lock().unwrap();
        let c = g.as_ref().unwrap();
        assert_eq!(pending_scenes(c, &b.id).unwrap(), vec![b.scenes[2].clone()]);
        scenes::scene_save(c, &b.scenes[0], &testkit::doc(&["Texto novo"]), 2, 5).unwrap();
        assert_eq!(pending_scenes(c, &b.id).unwrap(), vec![b.scenes[0].clone(), b.scenes[2].clone()]);
    }

    #[test]
    fn queue_respects_cancel_and_reports_progress() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let db = shared(c);
        let m = FakeModel::new(vec![text_turn("lixo"), text_turn(GOOD), text_turn(GOOD)]);
        let cancel = AtomicBool::new(false);
        let mut seen = vec![];
        run_queue(&db, &m, &b.id, &cancel, &mut |d, t| seen.push((d, t)), &now).unwrap();
        assert_eq!(seen.last(), Some(&(3, 3)));
        // a cena que falhou continua pendente
        assert_eq!(pending_scenes(db.lock().unwrap().as_ref().unwrap(), &b.id).unwrap(), vec![b.scenes[0].clone()]);
        let cancel = AtomicBool::new(true);
        let m2 = FakeModel::new(vec![]);
        run_queue(&db, &m2, &b.id, &cancel, &mut |_, _| {}, &now).unwrap();
        assert_eq!(m2.n_calls(), 0);
    }
}
