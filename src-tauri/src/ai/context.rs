//! Monta o que vai para a IA em cada pergunta: do mais estável ao mais variável.

use crate::ai::provider::Msg;
use crate::ai::text::plain_text;
use crate::db::{ai, books, scenes, DbError};
use rusqlite::Connection;

/// Mensagens recentes que vão na íntegra.
pub const WINDOW: usize = 12;
/// Acima disso (fora do resumo), as mais antigas são dobradas no resumo.
pub const FOLD_AT: usize = 24;

pub fn needs_fold(unfolded: usize) -> bool {
    unfolded > FOLD_AT
}

fn rules(mode: &str) -> &'static str {
    if mode == "coautora" {
        "Você é a Coautora deste livro. Converse em português do Brasil, de forma direta e útil. \
Quando pedirem texto para o livro, use a ferramenta propor_trecho: a proposta só entra no livro se a pessoa aprovar. \
Nunca diga que alterou o texto."
    } else {
        "Você é a Conselheira deste livro. Converse em português do Brasil, de forma direta e útil: sugira, pergunte, aponte furos. \
Você NÃO escreve no texto do livro e não tem como alterá-lo; se pedirem, explique isso e ofereça ideias ou exemplos na conversa."
    }
}

const COMMON: &str = "Regras: antes de afirmar algo sobre o livro, consulte (sumário, Bíblia, buscar_no_livro, ler_cena). \
Nunca invente fatos do livro; se não achar, diga que não encontrou. \
Quando surgir um fato novo e estável (personagem, lugar, acontecimento), registre com salvar_ficha. \
Seja econômica: para perguntas sobre o livro inteiro (análise, ritmo, arcos), use o sumário com os resumos e a Bíblia; leia uma cena inteira só quando precisar de um detalhe dela. Escreva em texto simples, sem Markdown (sem **, #, tabelas); listas curtas com hífen são ok. Os ids são só para as ferramentas: nunca mostre ids para a pessoa, cite cenas e capítulos pelo nome. \
O mapa de ideias (cartões ligados por setas) é da pessoa: leia com ler_mapa e, para sugerir cartões, use propor_cartoes; eles só entram se ela aprovar. Nunca diga que alterou o mapa.";

const IDEA: &str = "Esta conversa é para desenvolver a ideia de um livro novo. Ajude a pessoa a transformar a ideia em história: \
faça uma pergunta por vez (premissa, protagonista, o que ela quer, o que a impede, o mundo, o tom, um final possível), \
devolva em poucas linhas o que entendeu e, a cada ponto que firmar, use propor_cartoes com cartões curtos ligados entre si. \
Quando a história tiver forma, sugira um nome com propor_nome. Não escreva o livro nem cenas: o objetivo é a ideia.";

/// Bíblia em uma linha por ficha: `id · tipo · nome (apelidos): 1ª linha`.
pub fn bible_compact(c: &Connection, book_id: &str) -> Result<String, DbError> {
    let lines: Vec<String> = ai::bible_list(c, book_id)?
        .into_iter()
        .map(|e| {
            let first: String = e.body.lines().next().unwrap_or("").chars().take(140).collect();
            let al = if e.aliases.is_empty() { String::new() } else { format!(" ({})", e.aliases.join(", ")) };
            format!("{} · {} · {}{}: {}", e.id, e.kind, e.name, al, first)
        })
        .collect();
    Ok(if lines.is_empty() { "(vazia)".into() } else { lines.join("\n") })
}

/// Contexto completo: um system (instruções → Bíblia → mapa → cena atual → resumo) + janela recente.
pub fn build(c: &Connection, book_id: &str, current_scene_id: Option<&str>, conversation_id: &str) -> Result<Vec<Msg>, DbError> {
    let outline = books::book_outline(c, book_id)?;
    let mode = outline.book.ai_mode.clone();
    let mut map = String::new();
    for ch in &outline.chapters {
        map.push_str(&format!("## {}\n", ch.chapter.title));
        for sc in &ch.scenes {
            let summary = ai::digest_get(c, &sc.id)?.map(|d| d.1).unwrap_or_else(|| "(sem resumo)".into());
            map.push_str(&format!("- {} [id {}]: {}\n", sc.title, sc.id, summary));
        }
    }
    let current = match current_scene_id.and_then(|id| scenes::scene_get(c, id).ok()) {
        Some(s) => format!("# Cena aberta agora: {} [id {}]\n\n{}", s.title, s.id, plain_text(&s.content_json)),
        None => "# Nenhuma cena aberta agora.".into(),
    };
    let cv = ai::conversation_get(c, conversation_id)?;
    let summary = if cv.rolling_summary.is_empty() { String::new() } else { format!("\n\n# Resumo do início desta conversa\n{}", cv.rolling_summary) };
    let system = format!(
        "{}\n{}{}\nModo: {}.\n\n# Livro: {}\n\n# Bíblia (fichas)\n{}\n\n# Sumário do livro\n{}\n{}{}",
        rules(&mode),
        COMMON,
        if cv.purpose == "ideia" { format!("\n{IDEA}") } else { String::new() },
        if mode == "coautora" { "Coautora" } else { "Conselheira" },
        outline.book.title,
        bible_compact(c, book_id)?,
        map,
        current,
        summary
    );
    let mut msgs = vec![Msg::system(&system)];
    let chat: Vec<_> = ai::messages_all(c, conversation_id)?.into_iter().filter(|m| m.role != "note").collect();
    let unfolded = &chat[(cv.folded_count.max(0) as usize).min(chat.len())..];
    let start = unfolded.len().saturating_sub(WINDOW);
    for m in &unfolded[start..] {
        msgs.push(if m.role == "user" { Msg::user(&m.content) } else { Msg::assistant(&m.content) });
    }
    Ok(msgs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::testkit;
    use crate::db::{ai, test_conn};

    fn defs_text() -> String {
        crate::ai::tools::defs("conselheira", "livro").into_iter().map(|d| d.description).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn context_order_and_window() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        for i in 0..20 {
            ai::message_add(&c, &cv.id, if i % 2 == 0 { "user" } else { "assistant" }, &format!("m{i}"), None, i).unwrap();
            ai::message_add(&c, &cv.id, "note", "consultei", None, i).unwrap();
        }
        let msgs = build(&c, &b.id, Some(&b.scenes[0]), &cv.id).unwrap();
        assert_eq!(msgs[0].role, "system");
        assert!(msgs[0].content.contains("A tela do notebook"));
        let rest: Vec<_> = msgs[1..].iter().map(|m| m.content.as_str()).collect();
        assert_eq!(rest.len(), WINDOW);
        assert_eq!(rest[0], "m8");
        assert_eq!(*rest.last().unwrap(), "m19");
    }

    #[test]
    fn system_has_bible_map_and_mode_rules() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        ai::bible_upsert_ai(&c, &b.id, "personagem", "Rafa", &["Rafael".into()], "Programador\nmais", None, None, 1).unwrap();
        ai::digest_put(&c, &b.scenes[1], "h", "Rafa descobre a verdade.", 1).unwrap();
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        let s = &build(&c, &b.id, None, &cv.id).unwrap()[0].content;
        assert!(s.contains("Rafa (Rafael): Programador"));
        assert!(!s.contains("mais"));
        assert!(s.contains("Rafa descobre a verdade."));
        assert!(s.contains("(sem resumo)"));
        assert!(s.contains("Conselheira"));
        let i_bible = s.find("Rafa (Rafael)").unwrap();
        let i_map = s.find("Rafa descobre").unwrap();
        assert!(i_bible < i_map);
    }

    #[test]
    fn idea_conversation_gets_idea_instructions() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        assert!(!build(&c, &b.id, None, &cv.id).unwrap()[0].content.contains("uma pergunta por vez"));
        let idea = ai::conversation_new_idea(&c, &b.id, 2).unwrap();
        let s = &build(&c, &b.id, None, &idea.id).unwrap()[0].content;
        assert!(s.contains("uma pergunta por vez"));
        assert!(s.contains("propor_nome"));
    }

    #[test]
    fn outline_is_called_sumario() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        let s = &build(&c, &b.id, None, &cv.id).unwrap()[0].content;
        assert!(s.contains("# Sumário do livro"));
        assert!(!s.contains("Mapa do livro"));
        assert!(s.contains("propor_cartoes"));
        assert!(defs_text().contains("sumário"));
    }

    #[test]
    fn folded_messages_leave_window_and_summary_goes_in() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        for i in 0..4 {
            ai::message_add(&c, &cv.id, "user", &format!("m{i}"), None, i).unwrap();
        }
        ai::conversation_set_summary(&c, &cv.id, "Falamos do Rafa.", 2).unwrap();
        let msgs = build(&c, &b.id, None, &cv.id).unwrap();
        assert!(msgs[0].content.contains("Falamos do Rafa."));
        assert_eq!(msgs[1..].iter().map(|m| m.content.as_str()).collect::<Vec<_>>(), vec!["m2", "m3"]);
    }

    #[test]
    fn needs_fold_threshold() {
        assert!(!needs_fold(24));
        assert!(needs_fold(25));
    }
}
