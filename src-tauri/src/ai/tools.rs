//! Ferramentas que o agente da conversa pode chamar. Nenhuma escreve no manuscrito.

use crate::ai::provider::{ToolCall, ToolDef};
use crate::ai::text::plain_text;
use crate::db::ai::{self, fold, Proposal};
use crate::db::map::{self as mapdb, MapProposal};
use crate::db::{books, scenes};
use rusqlite::Connection;
use serde_json::{json, Value};

pub struct ToolOutcome {
    /// O que volta para o modelo.
    pub result: String,
    /// Linha discreta para a interface ("consultei a cena X"). Vazia = nada a mostrar.
    pub note: String,
    pub proposal: Option<Proposal>,
    /// Cartões ou nome sugeridos para o mapa, esperando aprovação.
    pub map_proposal: Option<MapProposal>,
}

fn def(name: &str, description: &str, parameters: Value) -> ToolDef {
    ToolDef { name: name.into(), description: description.into(), parameters }
}

/// Ferramentas oferecidas ao modelo. `propor_trecho` só existe no modo coautora;
/// `propor_nome`, só na conversa de desenvolver a ideia (`purpose == "ideia"`).
pub fn defs(mode: &str, purpose: &str) -> Vec<ToolDef> {
    let mut v = vec![
        def("buscar_no_livro", "Busca uma palavra ou expressão em todas as cenas do livro. Devolve trechos com a cena e o capítulo.",
            json!({"type":"object","properties":{"texto":{"type":"string"}},"required":["texto"]})),
        def("ler_cena", "Lê o texto completo de uma cena pelo id (os ids estão no sumário do livro).",
            json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"]})),
        def("ler_ficha", "Lê a ficha completa da Bíblia pelo id, nome ou apelido.",
            json!({"type":"object","properties":{"id_ou_nome":{"type":"string"}},"required":["id_ou_nome"]})),
        def("salvar_ficha", "Registra na Bíblia um fato novo e estável do livro (personagem, lugar, fato ou evento da linha do tempo). Use o id quando a ficha já existir.",
            json!({"type":"object","properties":{
                "id":{"type":"string"},
                "tipo":{"type":"string","enum":ai::KINDS},
                "nome":{"type":"string"},
                "apelidos":{"type":"array","items":{"type":"string"}},
                "fato":{"type":"string"},
                "cena_origem":{"type":"string"},
                "ordem":{"type":"integer","description":"Posição na linha do tempo da história (só para linha_do_tempo)."}
            },"required":["tipo","nome","fato"]})),
        def("ler_conversas_anteriores", "Título e resumo das conversas anteriores sobre este livro.",
            json!({"type":"object","properties":{}})),
        def("ler_mapa", "Lê o mapa de ideias da pessoa: os cartões (com id) e as setas entre eles.",
            json!({"type":"object","properties":{}})),
        def("propor_cartoes", "Sugere cartões (e setas) para o mapa de ideias. NÃO entram no mapa: aparecem para a pessoa aprovar. Use 'ref' curto para cada cartão novo; nas setas, 'de' e 'para' são um ref novo ou o id de um cartão que já está no mapa.",
            json!({"type":"object","properties":{
                "cartoes":{"type":"array","maxItems":mapdb::MAX_PROPOSED,"items":{"type":"object","properties":{"ref":{"type":"string"},"texto":{"type":"string","description":"Curto: primeira linha é o título do cartão."}},"required":["ref","texto"]}},
                "setas":{"type":"array","items":{"type":"object","properties":{"de":{"type":"string"},"para":{"type":"string"},"rotulo":{"type":"string"}},"required":["de","para"]}}
            },"required":["cartoes"]})),
    ];
    if purpose == "ideia" {
        v.push(def("propor_nome", "Sugere um nome para o livro. Só muda se a pessoa aceitar.",
            json!({"type":"object","properties":{"nome":{"type":"string"}},"required":["nome"]})));
    }
    // na conversa da ideia a IA não escreve o livro, em nenhum modo
    if mode == "coautora" && purpose != "ideia" {
        v.push(def("propor_trecho", "Propõe um trecho de texto para o livro. Ele NÃO entra no texto: aparece como proposta para a pessoa aprovar.",
            json!({"type":"object","properties":{"texto":{"type":"string"}},"required":["texto"]})));
    }
    v
}

fn out(result: impl Into<String>, note: impl Into<String>) -> ToolOutcome {
    ToolOutcome { result: result.into(), note: note.into(), proposal: None, map_proposal: None }
}

fn err(msg: impl std::fmt::Display) -> ToolOutcome {
    out(format!("Erro: {msg}"), "")
}

fn arg<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

/// Até 8 trechos de ~300 caracteres em volta das ocorrências.
fn search(c: &Connection, book_id: &str, query: &str) -> Result<String, crate::db::DbError> {
    let q: Vec<char> = fold(query).chars().collect();
    if q.is_empty() {
        return Ok("Erro: busca vazia".into());
    }
    let outline = books::book_outline(c, book_id)?;
    let mut hits = vec![];
    'outer: for ch in &outline.chapters {
        for sc in &ch.scenes {
            let text: Vec<char> = plain_text(&scenes::scene_get(c, &sc.id)?.content_json).chars().collect();
            let folded: Vec<char> = text.iter().map(|x| fold(&x.to_string()).chars().next().unwrap_or(*x)).collect();
            let mut i = 0;
            while i + q.len() <= folded.len() {
                if folded[i..i + q.len()] == q[..] {
                    let a = i.saturating_sub(150);
                    let b = (i + q.len() + 150).min(text.len());
                    let snippet: String = text[a..b].iter().collect();
                    hits.push(format!("[{} › {} · id {}] …{}…", ch.chapter.title, sc.title, sc.id, snippet.replace('\n', " ")));
                    if hits.len() >= 8 {
                        break 'outer;
                    }
                    i += q.len() + 150;
                } else {
                    i += 1;
                }
            }
        }
    }
    Ok(if hits.is_empty() { format!("Nada encontrado para \"{query}\".") } else { hits.join("\n") })
}

/// Executa uma chamada de ferramenta. Erros voltam ao modelo como texto, nunca como pânico.
pub fn run(c: &Connection, book_id: &str, mode: &str, current_scene_id: Option<&str>, conversation_id: &str, call: &ToolCall, now: i64) -> ToolOutcome {
    let args: Value = match serde_json::from_str(if call.arguments_json.trim().is_empty() { "{}" } else { &call.arguments_json }) {
        Ok(v) => v,
        Err(e) => return err(format!("argumentos inválidos: {e}")),
    };
    let r = (|| -> Result<ToolOutcome, crate::db::DbError> {
        Ok(match call.name.as_str() {
            "buscar_no_livro" => {
                let Some(q) = arg(&args, "texto") else { return Ok(err("falta 'texto'")) };
                out(search(c, book_id, q)?, format!("busquei \"{q}\" no livro"))
            }
            "ler_cena" => {
                let Some(id) = arg(&args, "id") else { return Ok(err("falta 'id'")) };
                let Ok(s) = scenes::scene_get(c, id) else { return Ok(err("cena não encontrada")) };
                let book_of: Option<String> = c
                    .query_row("SELECT book_id FROM chapters WHERE id=?1", [&s.chapter_id], |r| r.get(0))
                    .ok();
                if book_of.as_deref() != Some(book_id) {
                    return Ok(err("cena não encontrada"));
                }
                out(format!("# {}\n\n{}", s.title, plain_text(&s.content_json)), format!("consultei a cena \"{}\"", s.title))
            }
            "ler_ficha" => {
                let Some(key) = arg(&args, "id_ou_nome") else { return Ok(err("falta 'id_ou_nome'")) };
                let found = ai::bible_list(c, book_id)?
                    .into_iter()
                    .find(|e| e.id == key || fold(&e.name) == fold(key) || e.aliases.iter().any(|a| fold(a) == fold(key)));
                match found {
                    None => out(format!("Nenhuma ficha \"{key}\"."), ""),
                    Some(e) => {
                        let notes: Vec<String> = e.ai_notes.iter().map(|n| format!("- {}", n.fact)).collect();
                        out(
                            format!("{} ({}) [id {}]\nApelidos: {}\n{}\n{}", e.name, e.kind, e.id, e.aliases.join(", "), e.body,
                                if notes.is_empty() { String::new() } else { format!("Notas:\n{}", notes.join("\n")) }),
                            format!("consultei a ficha \"{}\"", e.name),
                        )
                    }
                }
            }
            "salvar_ficha" => {
                let (Some(kind), Some(name), Some(fact)) = (arg(&args, "tipo"), arg(&args, "nome"), arg(&args, "fato")) else {
                    return Ok(err("faltam 'tipo', 'nome' ou 'fato'"));
                };
                let aliases: Vec<String> = args.get("apelidos").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                let order = args.get("ordem").and_then(Value::as_i64);
                let source = arg(&args, "cena_origem").or(current_scene_id);
                // com id: usa o nome atual da ficha para cair nela mesmo que a IA mude o nome
                let name = match arg(&args, "id").and_then(|id| ai::bible_get(c, id).ok()) {
                    Some(e) if e.book_id == book_id => e.name,
                    _ => name.to_string(),
                };
                match ai::bible_upsert_ai(c, book_id, kind, &name, &aliases, fact, source, order, now) {
                    Ok(e) => out(format!("Ficha salva (id {}).", e.id), format!("atualizei a ficha \"{}\"", e.name)),
                    Err(crate::db::DbError::Invalid(m)) => err(m),
                    Err(e) => return Err(e),
                }
            }
            "ler_conversas_anteriores" => {
                let list: Vec<String> = ai::conversation_list(c, book_id)?
                    .into_iter()
                    .filter(|cv| cv.id != conversation_id && !cv.rolling_summary.is_empty())
                    .map(|cv| format!("- {}: {}", cv.title, cv.rolling_summary))
                    .collect();
                out(if list.is_empty() { "Não há conversas anteriores resumidas.".to_string() } else { list.join("\n") }, "li as conversas anteriores")
            }
            "propor_trecho" => {
                if mode != "coautora" {
                    return Ok(err("no modo Conselheira não é permitido propor texto para o livro"));
                }
                if ai::conversation_get(c, conversation_id)?.purpose == "ideia" {
                    return Ok(err("nesta conversa o objetivo é a ideia do livro: não proponha texto"));
                }
                let Some(text) = arg(&args, "texto") else { return Ok(err("falta 'texto'")) };
                let p = ai::proposal_add(c, book_id, Some(conversation_id), current_scene_id, text, now)?;
                ToolOutcome { result: "Proposta criada; a pessoa decide se entra no livro.".into(), note: String::new(), proposal: Some(p), map_proposal: None }
            }
            "ler_mapa" => {
                let m = mapdb::map_get(c, book_id)?;
                if m.cards.is_empty() {
                    out("(mapa vazio)", "consultei o mapa")
                } else {
                    let mut lines: Vec<String> = m.cards.iter().map(|k| {
                        let one: String = k.text.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(200).collect();
                        format!("- [{}] {}", k.id, one)
                    }).collect();
                    if !m.links.is_empty() {
                        lines.push("Setas:".into());
                        lines.extend(m.links.iter().map(|l| format!("{} → {}: {}", l.from_card, l.to_card, l.label)));
                    }
                    out(lines.join("\n"), "consultei o mapa")
                }
            }
            "propor_cartoes" => match mapdb::proposal_add_cards(c, book_id, Some(conversation_id), &args, now) {
                Ok(p) => ToolOutcome { result: "Sugestão criada; a pessoa decide o que entra no mapa.".into(), note: String::new(), proposal: None, map_proposal: Some(p) },
                Err(crate::db::DbError::Invalid(m)) => err(m),
                Err(e) => return Err(e),
            },
            "propor_nome" => {
                if ai::conversation_get(c, conversation_id)?.purpose != "ideia" {
                    return Ok(err("propor_nome só existe na conversa de desenvolver a ideia"));
                }
                let Some(nome) = arg(&args, "nome") else { return Ok(err("falta 'nome'")) };
                let p = mapdb::proposal_add_name(c, book_id, Some(conversation_id), nome, now)?;
                ToolOutcome { result: "Sugestão de nome criada; a pessoa decide.".into(), note: String::new(), proposal: None, map_proposal: Some(p) }
            }
            other => err(format!("ferramenta desconhecida: {other}")),
        })
    })();
    r.unwrap_or_else(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::provider::ToolCall;
    use crate::ai::testkit;
    use crate::db::{ai, test_conn};

    fn call(name: &str, args: serde_json::Value) -> ToolCall {
        ToolCall { id: "c".into(), name: name.into(), arguments_json: args.to_string() }
    }

    #[test]
    fn conselheira_has_no_propose_tool() {
        assert!(!defs("conselheira", "livro").iter().any(|d| d.name == "propor_trecho"));
        assert!(defs("coautora", "livro").iter().any(|d| d.name == "propor_trecho"));
    }

    #[test]
    fn run_rejects_propose_in_conselheira() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        let o = run(&c, &b.id, "conselheira", Some(&b.scenes[0]), &cv.id, &call("propor_trecho", serde_json::json!({"texto":"x"})), 1);
        assert!(o.proposal.is_none());
        assert!(o.result.contains("não"));
        assert!(ai::proposals_pending(&c, &b.id).unwrap().is_empty());
    }

    #[test]
    fn coautora_propose_creates_pending_proposal() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "coautora");
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        let o = run(&c, &b.id, "coautora", Some(&b.scenes[0]), &cv.id, &call("propor_trecho", serde_json::json!({"texto":"Novo trecho"})), 1);
        let p = o.proposal.unwrap();
        assert_eq!(p.scene_id.as_deref(), Some(b.scenes[0].as_str()));
        assert_eq!(ai::proposals_pending(&c, &b.id).unwrap().len(), 1);
    }

    #[test]
    fn search_finds_accented_word_with_context() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        let o = run(&c, &b.id, "conselheira", None, &cv.id, &call("buscar_no_livro", serde_json::json!({"texto":"ELEVADOR"})), 1);
        assert!(o.result.contains("A Porta do Elevador"));
        assert!(o.result.contains("descobre a verdade"));
        assert!(!o.note.is_empty());
    }

    #[test]
    fn read_scene_returns_text_and_note() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        let o = run(&c, &b.id, "conselheira", None, &cv.id, &call("ler_cena", serde_json::json!({"id": b.scenes[2]})), 1);
        assert!(o.result.contains("padaria"));
        assert!(o.note.contains("O Troco"));
    }

    #[test]
    fn save_card_respects_user_edit() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        let args = serde_json::json!({"tipo":"personagem","nome":"Rafa","fato":"Programador"});
        run(&c, &b.id, "conselheira", None, &cv.id, &call("salvar_ficha", args), 1);
        let e = &ai::bible_list(&c, &b.id).unwrap()[0];
        ai::bible_update_user(&c, &e.id, "Rafa", &[], "Meu texto", None, 2).unwrap();
        let args = serde_json::json!({"id": e.id, "tipo":"personagem","nome":"Rafa","fato":"Insone"});
        let o = run(&c, &b.id, "conselheira", None, &cv.id, &call("salvar_ficha", args), 3);
        let e = &ai::bible_list(&c, &b.id).unwrap()[0];
        assert_eq!(e.body, "Meu texto");
        assert_eq!(e.ai_notes.len(), 1);
        assert!(o.note.contains("Rafa"));
    }

    #[test]
    fn map_tools_offered_in_both_modes_and_name_only_in_idea() {
        for mode in ["conselheira", "coautora"] {
            let names: Vec<String> = defs(mode, "livro").into_iter().map(|d| d.name).collect();
            assert!(names.contains(&"ler_mapa".to_string()) && names.contains(&"propor_cartoes".to_string()));
            assert!(!names.contains(&"propor_nome".to_string()));
            assert!(defs(mode, "ideia").iter().any(|d| d.name == "propor_nome"));
        }
    }

    #[test]
    fn idea_conversation_never_offers_or_runs_propor_trecho() {
        assert!(!defs("coautora", "ideia").iter().any(|d| d.name == "propor_trecho"));
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "coautora");
        let idea = ai::conversation_new_idea(&c, &b.id, 1).unwrap();
        let o = run(&c, &b.id, "coautora", None, &idea.id, &call("propor_trecho", serde_json::json!({"texto":"x"})), 2);
        assert!(o.result.starts_with("Erro"));
        assert!(ai::proposals_pending(&c, &b.id).unwrap().is_empty());
    }

    #[test]
    fn propor_cartoes_works_in_conselheira() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        let args = serde_json::json!({"cartoes":[{"ref":"a","texto":"Marina"},{"ref":"b","texto":"A carta"}],"setas":[{"de":"a","para":"b","rotulo":"escreve"}]});
        let o = run(&c, &b.id, "conselheira", None, &cv.id, &call("propor_cartoes", args), 1);
        let p = o.map_proposal.expect("proposta");
        assert_eq!(p.kind, "cartoes");
        assert_eq!(crate::db::map::proposals_pending(&c, &b.id).unwrap().len(), 1);
        assert!(crate::db::map::map_get(&c, &b.id).unwrap().cards.is_empty(), "nada entra sem aprovação");
    }

    #[test]
    fn propor_cartoes_rejects_unknown_or_foreign_ids() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let other = crate::db::books::book_create(&c, "Outro", "conselheira", 0).unwrap();
        let foreign = crate::db::map::card_create(&c, &other.id, "x", "papel", 0.0, 0.0, 1).unwrap();
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        for to in ["nao-existe", foreign.id.as_str()] {
            let args = serde_json::json!({"cartoes":[{"ref":"a","texto":"Marina"}],"setas":[{"de":"a","para":to}]});
            let o = run(&c, &b.id, "conselheira", None, &cv.id, &call("propor_cartoes", args), 1);
            assert!(o.result.starts_with("Erro"), "{}", o.result);
            assert!(o.map_proposal.is_none());
        }
        assert!(crate::db::map::proposals_pending(&c, &b.id).unwrap().is_empty());
    }

    #[test]
    fn propor_nome_only_in_idea_conversation() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        let o = run(&c, &b.id, "conselheira", None, &cv.id, &call("propor_nome", serde_json::json!({"nome":"O Farol"})), 1);
        assert!(o.result.starts_with("Erro"));
        let idea = ai::conversation_new_idea(&c, &b.id, 2).unwrap();
        let o = run(&c, &b.id, "conselheira", None, &idea.id, &call("propor_nome", serde_json::json!({"nome":"O Farol"})), 3);
        assert_eq!(o.map_proposal.unwrap().kind, "nome");
    }

    #[test]
    fn ler_mapa_lists_cards_and_links() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        let o = run(&c, &b.id, "conselheira", None, &cv.id, &call("ler_mapa", serde_json::json!({})), 1);
        assert!(o.result.contains("(mapa vazio)"));
        let a = crate::db::map::card_create(&c, &b.id, "Marina\nfaroleira", "papel", 0.0, 0.0, 1).unwrap();
        let z = crate::db::map::card_create(&c, &b.id, "A carta", "papel", 0.0, 0.0, 1).unwrap();
        let l = crate::db::map::link_create(&c, &b.id, &a.id, &z.id).unwrap();
        crate::db::map::link_set_label(&c, &l.id, "escreve").unwrap();
        let o = run(&c, &b.id, "conselheira", None, &cv.id, &call("ler_mapa", serde_json::json!({})), 1);
        assert!(o.result.contains(&format!("- [{}] Marina faroleira", a.id)), "{}", o.result);
        assert!(o.result.contains(&format!("{} → {}: escreve", a.id, z.id)));
        assert_eq!(o.note, "consultei o mapa");
    }

    #[test]
    fn bad_arguments_return_error_not_panic() {
        let (_d, c) = test_conn();
        let b = testkit::sample(&c, "conselheira");
        let cv = ai::conversation_current(&c, &b.id, 1).unwrap();
        let o = run(&c, &b.id, "conselheira", None, &cv.id, &ToolCall { id: "c".into(), name: "ler_cena".into(), arguments_json: "{oops".into() }, 1);
        assert!(o.result.starts_with("Erro"));
        let o = run(&c, &b.id, "conselheira", None, &cv.id, &call("nao_existe", serde_json::json!({})), 1);
        assert!(o.result.starts_with("Erro"));
    }
}
