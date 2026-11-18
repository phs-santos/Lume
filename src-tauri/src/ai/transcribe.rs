//! Fila de transcrição dos áudios da caixa de ideias.

use super::chat::month_of;
use super::stt::{Transcriber, STT_MAX_BYTES, STT_MODEL};
use crate::db::{ai, inbox};
use crate::state::SharedDb;

pub const TOO_LONG: &str = "Áudio longo demais para transcrever por enquanto.";

/// Transcreve os áudios pendentes, um por vez. Tranca o banco só para ler o item e para gravar o resultado.
/// `t = None` (sem chave) marca os pendentes como `sem_chave`. Devolve quantos ficaram `pronta`.
pub fn transcribe_pending(db: &SharedDb, t: Option<&dyn Transcriber>, now: i64) -> Result<usize, String> {
    let Some(_guard) = RunGuard::take(db) else { return Ok(0) }; // já tem uma rodando para este banco
    let with = |f: &mut dyn FnMut(&rusqlite::Connection) -> Result<(), String>| -> Result<(), String> {
        let g = db.lock().unwrap();
        f(g.as_ref().ok_or_else(|| "LOCKED".to_string())?)
    };
    let err = |e: crate::db::DbError| e.to_string();

    let Some(t) = t else {
        with(&mut |c| inbox::inbox_mark_no_key(c).map(|_| ()).map_err(err))?;
        return Ok(0);
    };
    let (mut ids, mut vocab) = (vec![], vec![]);
    with(&mut |c| {
        ids = inbox::inbox_pending_audio(c).map_err(err)?;
        vocab = ai::bible_vocabulary(c).map_err(err)?;
        Ok(())
    })?;

    let mut ready = 0;
    for id in ids {
        let mut item: Option<(String, Vec<u8>)> = None;
        with(&mut |c| {
            let mime = inbox::inbox_list(c).map_err(err)?.into_iter().find(|i| i.id == id).map(|i| i.mime);
            if let Some(m) = mime {
                item = Some((m, inbox::inbox_body(c, &id).map_err(err)?));
            }
            Ok(())
        })?;
        let Some((mime, body)) = item else { continue }; // apagada no meio do caminho
        if body.len() > STT_MAX_BYTES {
            with(&mut |c| inbox::inbox_set_transcript(c, &id, "erro", None, Some(TOO_LONG)).map_err(err))?;
            continue;
        }
        // rede sem o banco trancado
        let result = t.transcribe(&body, &mime, &vocab);
        with(&mut |c| match &result {
            Ok(tr) => {
                inbox::inbox_set_transcript(c, &id, "pronta", Some(&tr.text), None).map_err(err)?;
                ai::usage_add(c, &month_of(now), STT_MODEL, tr.audio_ms, 0, None).map_err(err)
            }
            Err(e) => inbox::inbox_set_transcript(c, &id, "erro", None, Some(&e.to_string())).map_err(err),
        })?;
        if result.is_ok() {
            ready += 1;
        }
    }
    Ok(ready)
}

/// Uma transcrição por vez para cada banco (o painel e o ciclo de fundo podem pedir juntos).
struct RunGuard(usize);

static RUNNING: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());

impl RunGuard {
    fn take(db: &SharedDb) -> Option<RunGuard> {
        let key = std::sync::Arc::as_ptr(db) as usize;
        let mut r = RUNNING.lock().unwrap();
        if r.contains(&key) {
            return None;
        }
        r.push(key);
        Some(RunGuard(key))
    }
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        RUNNING.lock().unwrap().retain(|k| *k != self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::provider::ProviderError;
    use crate::ai::stt::Transcript;
    use crate::db::{books, test_conn};
    use std::sync::{Arc, Mutex};

    struct Fake {
        calls: Mutex<Vec<(Vec<u8>, String, Vec<String>)>>,
        result: Box<dyn Fn() -> Result<Transcript, ProviderError> + Send + Sync>,
        pause: Option<std::time::Duration>,
    }

    impl Fake {
        fn ok(text: &str) -> Self {
            let t = text.to_string();
            Fake { calls: Mutex::new(vec![]), result: Box::new(move || Ok(Transcript { text: t.clone(), audio_ms: 3000 })), pause: None }
        }
        fn failing(e: fn() -> ProviderError) -> Self {
            Fake { calls: Mutex::new(vec![]), result: Box::new(move || Err(e())), pause: None }
        }
        fn n(&self) -> usize {
            self.calls.lock().unwrap().len()
        }
    }

    impl Transcriber for Fake {
        fn transcribe(&self, audio: &[u8], mime: &str, vocabulary: &[String]) -> Result<Transcript, ProviderError> {
            if let Some(p) = self.pause {
                std::thread::sleep(p);
            }
            self.calls.lock().unwrap().push((audio.to_vec(), mime.into(), vocabulary.to_vec()));
            (self.result)()
        }
    }

    fn shared() -> (tempfile::TempDir, SharedDb) {
        let (d, c) = test_conn();
        (d, Arc::new(Mutex::new(Some(c))))
    }

    fn with<T>(db: &SharedDb, f: impl FnOnce(&rusqlite::Connection) -> T) -> T {
        f(db.lock().unwrap().as_ref().unwrap())
    }

    fn item(db: &SharedDb, id: &str) -> inbox::InboxItem {
        with(db, |c| inbox::inbox_list(c).unwrap().into_iter().find(|i| i.id == id).unwrap())
    }

    const NOW: i64 = 1_763_400_000_000; // novembro de 2025

    #[test]
    fn pending_audio_becomes_ready_and_usage_counts_ms() {
        let (_d, db) = shared();
        with(&db, |c| {
            inbox::inbox_insert(c, "a", "audio", "audio/mp4", &[1, 2], 1, 1).unwrap();
            inbox::inbox_insert(c, "b", "audio", "audio/webm", &[3], 2, 1).unwrap();
        });
        let f = Fake::ok("E se a Marina fugir?");
        assert_eq!(transcribe_pending(&db, Some(&f), NOW), Ok(2));
        assert_eq!(item(&db, "a").transcript.as_deref(), Some("E se a Marina fugir?"));
        assert_eq!(item(&db, "b").transcript_status.as_deref(), Some("pronta"));
        let usage = with(&db, |c| ai::usage_month(c, &month_of(NOW)).unwrap());
        let stt = usage.iter().find(|u| u.model == STT_MODEL).unwrap();
        assert_eq!(stt.input_tokens, 6000);
    }

    #[test]
    fn failure_marks_error_once_and_keeps_audio() {
        let (_d, db) = shared();
        with(&db, |c| inbox::inbox_insert(c, "a", "audio", "audio/mp4", &[9, 9], 1, 1).unwrap());
        let f = Fake::failing(|| ProviderError::Auth);
        assert_eq!(transcribe_pending(&db, Some(&f), NOW), Ok(0));
        assert_eq!(f.n(), 1);
        let i = item(&db, "a");
        assert_eq!(i.transcript_status.as_deref(), Some("erro"));
        assert_eq!(i.transcript_error.as_deref(), Some("Chave da Inworld inválida"));
        assert_eq!(with(&db, |c| inbox::inbox_body(c, "a").unwrap()), vec![9, 9]);
    }

    #[test]
    fn too_long_audio_is_error_without_calling() {
        let (_d, db) = shared();
        with(&db, |c| inbox::inbox_insert(c, "a", "audio", "audio/mp4", &vec![0u8; STT_MAX_BYTES + 1], 1, 1).unwrap());
        let f = Fake::ok("x");
        transcribe_pending(&db, Some(&f), NOW).unwrap();
        assert_eq!(f.n(), 0);
        assert_eq!(item(&db, "a").transcript_error.as_deref(), Some(TOO_LONG));
    }

    #[test]
    fn without_key_marks_no_key() {
        let (_d, db) = shared();
        with(&db, |c| inbox::inbox_insert(c, "a", "audio", "audio/mp4", &[1], 1, 1).unwrap());
        assert_eq!(transcribe_pending(&db, None, NOW), Ok(0));
        assert_eq!(item(&db, "a").transcript_status.as_deref(), Some("sem_chave"));
    }

    #[test]
    fn ready_audio_is_not_transcribed_again() {
        let (_d, db) = shared();
        with(&db, |c| {
            inbox::inbox_insert(c, "a", "audio", "audio/mp4", &[1], 1, 1).unwrap();
            inbox::inbox_set_transcript(c, "a", "pronta", Some("já"), None).unwrap();
        });
        let f = Fake::ok("de novo");
        transcribe_pending(&db, Some(&f), NOW).unwrap();
        assert_eq!(f.n(), 0);
    }

    #[test]
    fn audio_without_status_from_older_app_is_treated_as_pending() {
        let (_d, db) = shared();
        with(&db, |c| {
            c.execute("INSERT INTO inbox(id,kind,mime,body,created_at,received_at,status) VALUES('velho','audio','audio/mp4',x'01',1,1,'nova')", []).unwrap();
        });
        let f = Fake::ok("Ideia antiga");
        assert_eq!(transcribe_pending(&db, Some(&f), NOW), Ok(1));
        assert_eq!(item(&db, "velho").transcript.as_deref(), Some("Ideia antiga"));
    }

    #[test]
    fn bible_vocabulary_and_mime_reach_the_transcriber() {
        let (_d, db) = shared();
        with(&db, |c| {
            let b = books::book_create(c, "L", "conselheira", 0).unwrap();
            ai::bible_upsert_ai(c, &b.id, "personagem", "Teodoro", &[], "f", None, None, 1).unwrap();
            inbox::inbox_insert(c, "a", "audio", "audio/webm;codecs=opus", &[1], 1, 1).unwrap();
        });
        let f = Fake::ok("x");
        transcribe_pending(&db, Some(&f), NOW).unwrap();
        let calls = f.calls.lock().unwrap();
        assert_eq!(calls[0].1, "audio/webm;codecs=opus");
        assert_eq!(calls[0].2, ["Teodoro"]);
    }

    #[test]
    fn second_call_while_running_does_nothing() {
        let (_d, db) = shared();
        with(&db, |c| inbox::inbox_insert(c, "a", "audio", "audio/mp4", &[1], 1, 1).unwrap());
        let slow = Arc::new(Fake { pause: Some(std::time::Duration::from_millis(200)), ..Fake::ok("x") });
        let (db2, f2) = (db.clone(), slow.clone());
        let h = std::thread::spawn(move || transcribe_pending(&db2, Some(f2.as_ref()), NOW));
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(transcribe_pending(&db, Some(slow.as_ref()), NOW), Ok(0));
        assert_eq!(h.join().unwrap(), Ok(1));
        assert_eq!(slow.n(), 1);
    }
}
