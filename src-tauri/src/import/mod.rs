//! Importação de livros prontos para dentro do Lume.
pub mod epub;

use crate::db::{books, scenes, DbError};
use epub::{scene_doc, scene_text, ImportedBook};
use rusqlite::Connection;

/// Grava um livro importado: um capítulo do Lume por capítulo do EPUB, uma cena por trecho.
pub fn import_book(c: &Connection, b: &ImportedBook, ai_mode: &str, now: i64) -> Result<books::Book, DbError> {
    let tx = crate::db::Atomic::begin(c)?;
    let book = books::book_create(c, &b.title, ai_mode, now)?;
    // book_create já cria "Capítulo 1 / Cena 1": troca pelo conteúdo do EPUB
    let starter = books::book_outline(c, &book.id)?;
    for ch in &b.chapters {
        let chapter = books::chapter_create(c, &book.id, &ch.title, None)?;
        for (i, sc) in ch.scenes.iter().enumerate() {
            let title = if ch.scenes.len() == 1 { ch.title.clone() } else { format!("Cena {}", i + 1) };
            let s = scenes::scene_create(c, &chapter.id, &title, None, now)?;
            let words = crate::db::inbox::count_words(&scene_text(sc));
            scenes::scene_save(c, &s.id, &scene_doc(sc).to_string(), words, now)?;
        }
    }
    for old in starter.chapters {
        for s in old.scenes {
            scenes::scene_delete(c, &s.id)?;
        }
        books::chapter_delete(c, &old.chapter.id)?;
    }
    tx.commit()?;
    Ok(book)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn imported_book_has_chapters_scenes_and_word_counts() {
        let (_d, c) = test_conn();
        let parsed = epub::parse_epub(&epub::tests::sample_epub()).unwrap();
        let b = import_book(&c, &parsed, "conselheira", 7).unwrap();
        assert_eq!(b.title, "Até ter certeza");
        let o = books::book_outline(&c, &b.id).unwrap();
        let shape: Vec<(String, usize)> = o.chapters.iter().map(|x| (x.chapter.title.clone(), x.scenes.len())).collect();
        assert_eq!(shape, [("Linha 340".to_string(), 1), ("A carta".to_string(), 3)]);
        let first = &o.chapters[0].scenes[0];
        assert_eq!(first.title, "Linha 340");
        assert_eq!(first.word_count, 19);
        assert_eq!(o.chapters[1].scenes.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(), ["Cena 1", "Cena 2", "Cena 3"]);
        let s = scenes::scene_get(&c, &first.id).unwrap();
        assert!(s.content_json.contains("\"italic\""));
    }

    #[test]
    fn failed_import_leaves_nothing_behind() {
        let (_d, c) = test_conn();
        let parsed = epub::parse_epub(&epub::tests::sample_epub()).unwrap();
        assert!(import_book(&c, &parsed, "modo-invalido", 1).is_err());
        assert!(books::book_list(&c).unwrap().is_empty());
    }
}
