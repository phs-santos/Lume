//! Mostra como um EPUB vai ficar no Lume (só leitura).
use lume_lib::import::epub::{parse_epub, scene_text};

fn main() {
    let path = std::env::args().nth(1).expect("caminho do .epub");
    let b = parse_epub(&std::fs::read(path).unwrap()).unwrap();
    println!("título: {}", b.title);
    let mut total = 0;
    for c in &b.chapters {
        let words: usize = c.scenes.iter().map(|s| scene_text(s).split_whitespace().count()).sum();
        let italics = c.scenes.iter().flat_map(|s| s.paragraphs.iter().flatten().flatten()).filter(|r| r.italic).count();
        total += words;
        println!("{:<28} cenas:{} parágrafos:{:>3} palavras:{:>5} itálicos:{}", c.title, c.scenes.len(), c.scenes.iter().map(|s| s.paragraphs.len()).sum::<usize>(), words, italics);
    }
    println!("total: {} capítulos, ~{} palavras", b.chapters.len(), total);
    let first = &b.chapters[0].scenes[0].paragraphs[0];
    println!("1º parágrafo: {}", first.iter().flatten().map(|r| r.text.as_str()).collect::<String>().chars().take(90).collect::<String>());
}
