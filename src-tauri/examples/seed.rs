//! Cria um cofre com livros de exemplo, para testar o app com conteúdo.
//! Uso: SEED_PASSWORD=... cargo run --example seed -- <pasta-de-dados> <apelido> <arquivo-da-frase>

use lume_lib::{commands, db, state::AppState};
use serde_json::json;
use std::path::PathBuf;

fn doc(paragraphs: &[&str]) -> String {
    let content: Vec<_> = paragraphs
        .iter()
        .map(|p| json!({"type": "paragraph", "content": [{"type": "text", "text": p}]}))
        .collect();
    json!({"type": "doc", "content": content}).to_string()
}

fn words(paragraphs: &[&str]) -> i64 {
    paragraphs.iter().map(|p| p.split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count() as i64).sum()
}

type Scene<'a> = (&'a str, &'a [&'a str]);
type Chapter<'a> = (&'a str, &'a [Scene<'a>]);

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(args.next().expect("pasta de dados"));
    let nick = args.next().expect("apelido");
    let phrase_file = PathBuf::from(args.next().expect("arquivo da frase"));
    let password = std::env::var("SEED_PASSWORD").expect("SEED_PASSWORD");

    let state = AppState::new(dir);
    let phrase = commands::create_vault(&state, &nick, &password).expect("criar cofre");
    std::fs::create_dir_all(phrase_file.parent().unwrap()).unwrap();
    std::fs::write(&phrase_file, format!("{phrase}\n")).unwrap();

    let books: &[(&str, &str, &[Chapter])] = &[
        ("A Carta da Marina", "conselheira", &[
            ("Capítulo 1", &[
                ("A receita da avó", &[
                    "Marina encontrou a carta dentro do livro de receitas da avó, entre duas páginas coladas de açúcar.",
                    "O envelope não tinha selo. Só um nome, escrito com a letra inclinada de quem tinha pressa: Teodoro.",
                    "— Isso não é seu — disse o irmão, da porta.",
                ]),
                ("O irmão na porta", &[
                    "Caio não entrou na cozinha. Ficou encostado no batente, os braços cruzados, olhando a carta como se ela pudesse morder.",
                    "— Você sabia — ela disse. Não era uma pergunta.",
                ]),
            ]),
            ("Capítulo 2", &[
                ("Teodoro", &[
                    "Na manhã seguinte, a cozinha cheirava a café queimado. Ninguém falou da carta.",
                    "Mas à tarde Marina já tinha encontrado o endereço, e às seis estava no ônibus para a serra.",
                ]),
            ]),
        ]),
        ("O Farol de Itaipava", "coautora", &[
            ("Capítulo 1", &[
                ("Chegada", &[
                    "O farol estava apagado havia onze anos, e mesmo assim os pescadores ainda desviavam dele.",
                    "Helena chegou com duas malas e uma chave de ferro que pesava mais do que deveria.",
                ]),
            ]),
        ]),
        ("Cartas que Não Mandei", "conselheira", &[
            ("Para minha mãe", &[
                ("Março", &[
                    "Mãe, hoje eu aprendi a fazer o seu arroz. Ficou empapado, do jeito que você dizia que eu ia fazer.",
                ]),
            ]),
            ("Para o Rafael", &[
                ("Sem data", &[
                    "Eu nunca te disse que fui eu quem quebrou o rádio. Agora você sabe. Não muda nada, mas eu queria que soubesse.",
                ]),
            ]),
        ]),
        ("Verão em Paraty", "conselheira", &[
            ("Capítulo 1", &[
                ("A pousada", &[
                    "As pedras da rua eram tão irregulares que a mala de rodinhas desistiu antes dela.",
                    "A dona da pousada riu da cena inteira, e foi assim que as duas viraram amigas.",
                ]),
            ]),
        ]),
    ];

    let guard = state.db.lock().unwrap();
    let c = guard.as_ref().unwrap();
    let mut t = 1_763_000_000_000i64;
    for (title, mode, chapters) in books {
        t += 60_000;
        let b = db::books::book_create(c, title, mode, t).unwrap();
        let o = db::books::book_outline(c, &b.id).unwrap();
        // remove o capítulo/cena padrão e cria os do exemplo
        let first_ch = o.chapters[0].chapter.id.clone();
        db::scenes::scene_delete(c, &o.chapters[0].scenes[0].id).unwrap();
        db::books::chapter_delete(c, &first_ch).unwrap();
        for (ch_title, scenes) in chapters.iter() {
            let ch = db::books::chapter_create(c, &b.id, ch_title, None).unwrap();
            for (sc_title, paragraphs) in scenes.iter() {
                let s = db::scenes::scene_create(c, &ch.id, sc_title, None, t).unwrap();
                db::scenes::scene_save(c, &s.id, &doc(paragraphs), words(paragraphs), t).unwrap();
            }
        }
        println!("livro criado: {title}");
    }
}
