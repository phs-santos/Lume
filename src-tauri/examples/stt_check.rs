//! Teste real da transcrição com a chave guardada no cofre (a chave não é impressa).
//! Uso: VAULT_PASSWORD=... cargo run --example stt_check -- <pasta-de-dados> <audio> <mime>
use lume_lib::{ai::stt::{InworldStt, Transcriber}, commands, db, state::AppState};

fn main() {
    let mut a = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(a.next().unwrap());
    let (file, mime) = (a.next().unwrap(), a.next().unwrap());
    let state = AppState::new(dir);
    commands::unlock_vault(&state, &std::env::var("VAULT_PASSWORD").unwrap()).expect("abrir o cofre");
    let (key, vocab) = {
        let g = state.db.lock().unwrap();
        let c = g.as_ref().unwrap();
        let key = db::settings::setting_get(c, "ai.inworld_key").unwrap().expect("sem chave da IA no cofre");
        (key, db::ai::bible_vocabulary(c).unwrap())
    };
    println!("vocabulário da Bíblia: {} termos", vocab.len());
    let audio = std::fs::read(file).unwrap();
    match (InworldStt { key }).transcribe(&audio, &mime, &vocab) {
        Ok(t) => println!("transcrição ({} ms de áudio): {}", t.audio_ms, t.text),
        Err(e) => println!("erro: {e}"),
    }
}
