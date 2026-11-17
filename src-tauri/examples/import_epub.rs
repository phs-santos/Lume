//! Importa um .epub direto no cofre de uma pessoa (com o app fechado).
//! Uso: VAULT_PASSWORD=... cargo run --example import_epub -- <pasta-de-dados> <arquivo.epub>
use lume_lib::{commands, db, import, state::AppState};

fn main() {
    let mut a = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(a.next().expect("pasta de dados"));
    let file = a.next().expect("arquivo .epub");
    let state = AppState::new(dir);
    commands::unlock_vault(&state, &std::env::var("VAULT_PASSWORD").unwrap()).expect("abrir o cofre");
    let parsed = import::epub::parse_epub(&std::fs::read(file).unwrap()).expect("ler o epub");
    let g = state.db.lock().unwrap();
    let c = g.as_ref().unwrap();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
    let b = import::import_book(c, &parsed, "conselheira", now).expect("gravar");
    let o = db::books::book_outline(c, &b.id).unwrap();
    let words: i64 = o.chapters.iter().flat_map(|x| &x.scenes).map(|s| s.word_count).sum();
    println!("importado: {} — {} capítulos, {} palavras", b.title, o.chapters.len(), words);
    println!("livros na biblioteca: {:?}", db::books::book_list(c).unwrap().iter().map(|b| b.title.clone()).collect::<Vec<_>>());
}
