//! Laço da conversa: contexto → modelo → ferramentas → resposta, com limite e memória.

use crate::ai::context::{self, needs_fold, WINDOW};
use crate::ai::provider::{ChatModel, Msg, Usage};
use crate::ai::{pricing, tools};
use crate::db::ai::{self, Proposal};
use crate::db::settings::setting_get;
use crate::db::{books, DbError};
use crate::state::SharedDb;
use rusqlite::Connection;
use serde::Serialize;

pub const MAX_TOOL_CALLS: usize = 6;
pub const DEFAULT_CHAT_MODEL: &str = "anthropic/claude-sonnet-5-5";
pub const DEFAULT_BACKGROUND_MODEL: &str = "anthropic/claude-haiku-5-5";

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEvent {
    Delta { text: String },
    Note { text: String },
    Proposal { proposal: Proposal },
    /// fim da resposta, com o gasto dela (custo estimado; None se o modelo não está na tabela)
    Done { message_id: String, input_tokens: i64, output_tokens: i64, cost: Option<f64> },
    Error { message: String },
}

#[derive(Debug, Clone)]
pub struct AiConfig {
    pub chat_model: String,
    pub background_model: String,
}

impl AiConfig {
    pub fn load(c: &Connection) -> Result<Self, DbError> {
        let get = |k: &str, d: &str| -> Result<String, DbError> {
            Ok(setting_get(c, k)?.filter(|v| !v.trim().is_empty()).unwrap_or_else(|| d.to_string()))
        };
        Ok(Self { chat_model: get("ai.chat_model", DEFAULT_CHAT_MODEL)?, background_model: get("ai.background_model", DEFAULT_BACKGROUND_MODEL)? })
    }
}

/// "AAAA-MM" de um instante em milissegundos (UTC).
pub fn month_of(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    // algoritmo civil_from_days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}")
}

fn with<T>(db: &SharedDb, f: impl FnOnce(&Connection) -> Result<T, DbError>) -> Result<T, String> {
    let g = db.lock().unwrap();
    let c = g.as_ref().ok_or_else(|| "LOCKED".to_string())?;
    f(c).map_err(|e| e.to_string())
}

/// Grava o gasto; sem custo do provedor, usa a estimativa da tabela. Devolve o custo gravado.
pub fn record_usage(db: &SharedDb, model: &str, u: &Usage, now: i64) -> Option<f64> {
    let cost = u.cost.or_else(|| pricing::estimate(model, u.input_tokens, u.output_tokens));
    let _ = with(db, |c| ai::usage_add(c, &month_of(now), model, u.input_tokens, u.output_tokens, cost));
    cost
}

/// Limite mensal em US$ (ajuste "ai.monthly_limit"); None = sem limite.
pub fn monthly_limit(c: &Connection) -> Result<Option<f64>, DbError> {
    Ok(setting_get(c, "ai.monthly_limit")?.and_then(|v| v.replace(',', ".").trim().parse::<f64>().ok()).filter(|v| *v > 0.0))
}

/// Mensagem de erro se o gasto do mês já chegou ao limite.
pub fn limit_reached(c: &Connection, now: i64) -> Result<Option<String>, DbError> {
    let Some(limit) = monthly_limit(c)? else { return Ok(None) };
    let spent = ai::usage_cost_month(c, &month_of(now))?;
    Ok((spent >= limit).then(|| format!("O gasto deste mês chegou ao limite de US$ {limit:.2}. Aumente ou tire o limite em Ajustes.")))
}

/// Grava a pergunta e responde. A pergunta fica salva mesmo se a IA falhar.
pub fn send(db: &SharedDb, model: &dyn ChatModel, book_id: &str, scene_id: Option<&str>, text: &str, emit: &mut dyn FnMut(ChatEvent), now: &dyn Fn() -> i64) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Mensagem vazia".into());
    }
    let conv_id = with(db, |c| {
        let cv = ai::conversation_current(c, book_id, now())?;
        if cv.title == "Conversa" && ai::messages_all(c, &cv.id)?.is_empty() {
            let t: String = text.chars().take(48).collect();
            ai::conversation_set_title(c, &cv.id, &t)?;
        }
        ai::message_add(c, &cv.id, "user", text, None, now())?;
        Ok(cv.id)
    })?;
    // só depois de a pergunta estar salva: dobra mensagens antigas no resumo (falha não impede a resposta)
    let _ = fold_if_needed(db, model, book_id, now);
    answer(db, model, book_id, scene_id, &conv_id, emit, now)
}

/// Responde de novo à última pergunta (depois de um erro), sem duplicá-la.
pub fn retry(db: &SharedDb, model: &dyn ChatModel, book_id: &str, scene_id: Option<&str>, emit: &mut dyn FnMut(ChatEvent), now: &dyn Fn() -> i64) -> Result<(), String> {
    let conv_id = with(db, |c| {
        let cv = ai::conversation_current(c, book_id, now())?;
        let last = ai::messages_all(c, &cv.id)?.into_iter().filter(|m| m.role != "note").last();
        Ok(last.filter(|m| m.role == "user").map(|_| cv.id))
    })?
    .ok_or_else(|| "Não há pergunta para repetir".to_string())?;
    answer(db, model, book_id, scene_id, &conv_id, emit, now)
}

fn answer(db: &SharedDb, model: &dyn ChatModel, book_id: &str, scene_id: Option<&str>, conv_id: &str, emit: &mut dyn FnMut(ChatEvent), now: &dyn Fn() -> i64) -> Result<(), String> {
    if let Some(message) = with(db, |c| limit_reached(c, now()))? {
        emit(ChatEvent::Error { message: message.clone() });
        return Err(message);
    }
    let (cfg, mode) = with(db, |c| Ok((AiConfig::load(c)?, books::book_outline(c, book_id)?.book.ai_mode)))?;
    let mut extra: Vec<Msg> = vec![];
    let (mut tin, mut tout, mut cost) = (0i64, 0i64, Some(0f64));
    let mut used = 0usize;
    let mut full_text = String::new();
    loop {
        let base = with(db, |c| context::build(c, book_id, scene_id, conv_id))?;
        let msgs: Vec<Msg> = base.into_iter().chain(extra.iter().cloned()).collect();
        let offered = if used >= MAX_TOOL_CALLS { vec![] } else { tools::defs(&mode) };
        // o lock do banco NÃO fica preso durante a chamada de rede
        let turn = match model.complete(&cfg.chat_model, &msgs, &offered, &mut |d| emit(ChatEvent::Delta { text: d.to_string() })) {
            Ok(t) => t,
            Err(e) => {
                let message = e.to_string();
                emit(ChatEvent::Error { message: message.clone() });
                return Err(message);
            }
        };
        let c = record_usage(db, &cfg.chat_model, &turn.usage, now());
        tin += turn.usage.input_tokens;
        tout += turn.usage.output_tokens;
        cost = cost.zip(c).map(|(a, b)| a + b);
        full_text.push_str(&turn.text);
        if turn.tool_calls.is_empty() || offered.is_empty() {
            let text = if full_text.trim().is_empty() { "(sem resposta)".to_string() } else { full_text.trim().to_string() };
            let m = with(db, |c| ai::message_add(c, conv_id, "assistant", &text, None, now()))?;
            emit(ChatEvent::Done { message_id: m.id, input_tokens: tin, output_tokens: tout, cost });
            return Ok(());
        }
        extra.push(Msg::assistant_tools(&turn.text, turn.tool_calls.clone()));
        for call in &turn.tool_calls {
            if used >= MAX_TOOL_CALLS {
                extra.push(Msg::tool(&call.id, "Limite de consultas desta resposta atingido. Responda com o que já tem."));
                continue;
            }
            used += 1;
            let o = with(db, |c| {
                let o = tools::run(c, book_id, &mode, scene_id, conv_id, call, now());
                if !o.note.is_empty() {
                    ai::message_add(c, conv_id, "note", &o.note, Some(&call.name), now())?;
                }
                Ok(o)
            })?;
            if !o.note.is_empty() {
                emit(ChatEvent::Note { text: o.note.clone() });
            }
            if let Some(p) = o.proposal.clone() {
                emit(ChatEvent::Proposal { proposal: p });
            }
            extra.push(Msg::tool(&call.id, &o.result));
        }
    }
}

/// Quando há mensagens demais fora do resumo, dobra as mais antigas no resumo (modelo de fundo).
pub fn fold_if_needed(db: &SharedDb, model: &dyn ChatModel, book_id: &str, now: &dyn Fn() -> i64) -> Result<(), String> {
    let (cfg, cv, chat) = with(db, |c| {
        let cv = ai::conversation_current(c, book_id, now())?;
        let chat: Vec<_> = ai::messages_all(c, &cv.id)?.into_iter().filter(|m| m.role != "note").collect();
        Ok((AiConfig::load(c)?, cv, chat))
    })?;
    let folded = (cv.folded_count.max(0) as usize).min(chat.len());
    if !needs_fold(chat.len() - folded) {
        return Ok(());
    }
    let upto = chat.len() - WINDOW;
    let transcript: String = chat[folded..upto]
        .iter()
        .map(|m| format!("{}: {}\n", if m.role == "user" { "Pessoa" } else { "IA" }, m.content))
        .collect();
    let prompt = format!(
        "Resumo atual da conversa (pode estar vazio):\n{}\n\nMensagens novas:\n{}\n\nEscreva o resumo atualizado, em português, em no máximo 200 palavras: decisões tomadas, ideias combinadas, dúvidas abertas e pedidos da pessoa. Só o resumo.",
        cv.rolling_summary, transcript
    );
    let turn = model
        .complete(&cfg.background_model, &[Msg::user(&prompt)], &[], &mut |_| {})
        .map_err(|e| e.to_string())?;
    record_usage(db, &cfg.background_model, &turn.usage, now());
    if turn.text.trim().is_empty() {
        return Err("resumo vazio".into());
    }
    with(db, |c| ai::conversation_set_summary(c, &cv.id, turn.text.trim(), upto as i64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::testkit::{self, shared, text_turn, tool_turn, FakeModel};
    use crate::ai::provider::ProviderError;
    use crate::db::{ai, test_conn};

    fn now() -> i64 { 1_794_900_000_000 } // nov/2026

    fn setup(mode: &str) -> (tempfile::TempDir, crate::state::SharedDb, testkit::Book) {
        let (d, c) = test_conn();
        let b = testkit::sample(&c, mode);
        (d, shared(c), b)
    }

    fn msgs(db: &crate::state::SharedDb, book: &str) -> Vec<(String, String)> {
        let g = db.lock().unwrap();
        let c = g.as_ref().unwrap();
        let cv = ai::conversation_current(c, book, 0).unwrap();
        ai::messages_all(c, &cv.id).unwrap().into_iter().map(|m| (m.role, m.content)).collect()
    }

    #[test]
    fn plain_answer_streams_and_saves() {
        let (_d, db, b) = setup("conselheira");
        let m = FakeModel::new(vec![text_turn("Olá, tudo bem")]);
        let mut ev = vec![];
        send(&db, &m, &b.id, Some(&b.scenes[0]), "oi", &mut |e| ev.push(e), &now).unwrap();
        assert!(ev.iter().filter(|e| matches!(e, ChatEvent::Delta { .. })).count() >= 2);
        assert!(matches!(ev.last(), Some(ChatEvent::Done { .. })));
        assert_eq!(msgs(&db, &b.id), vec![("user".into(), "oi".into()), ("assistant".into(), "Olá, tudo bem".into())]);
    }

    #[test]
    fn tool_then_answer_emits_note_and_saves_once() {
        let (_d, db, b) = setup("conselheira");
        let m = FakeModel::new(vec![tool_turn("ler_cena", serde_json::json!({"id": b.scenes[1]})), text_turn("Ele descobre no elevador.")]);
        let mut ev = vec![];
        send(&db, &m, &b.id, None, "quando o Rafa descobre?", &mut |e| ev.push(e), &now).unwrap();
        assert!(ev.iter().any(|e| matches!(e, ChatEvent::Note { text } if text.contains("A Porta do Elevador"))));
        let all = msgs(&db, &b.id);
        assert_eq!(all.iter().filter(|(r, _)| r == "assistant").count(), 1);
        assert_eq!(all.iter().filter(|(r, _)| r == "note").count(), 1);
        // a 2ª chamada recebeu o resultado da ferramenta
        let calls = m.calls.lock().unwrap();
        assert!(calls[1].3.contains("descobre a verdade"));
    }

    #[test]
    fn tool_loop_capped_at_6_then_final_without_tools() {
        let (_d, db, b) = setup("conselheira");
        let mut m = FakeModel::new(vec![]);
        m.always_tool = true;
        send(&db, &m, &b.id, None, "pergunta", &mut |_| {}, &now).unwrap();
        let calls = m.calls.lock().unwrap();
        assert_eq!(calls.len(), MAX_TOOL_CALLS + 1);
        assert_eq!(calls.last().unwrap().2, 0);
        assert!(calls[0].2 > 0);
    }

    #[test]
    fn provider_error_keeps_user_message_and_saves_no_answer() {
        let (_d, db, b) = setup("conselheira");
        let m = FakeModel::new(vec![Err(ProviderError::Quota)]);
        let mut ev = vec![];
        assert!(send(&db, &m, &b.id, None, "oi", &mut |e| ev.push(e), &now).is_err());
        assert!(matches!(ev.last(), Some(ChatEvent::Error { message }) if message.contains("crédito")));
        assert_eq!(msgs(&db, &b.id), vec![("user".into(), "oi".into())]);
    }

    #[test]
    fn done_reports_tokens_and_estimated_cost() {
        let (_d, db, b) = setup("conselheira");
        let m = FakeModel::new(vec![tool_turn("ler_cena", serde_json::json!({"id": b.scenes[1]})), text_turn("ok")]);
        let mut ev = vec![];
        send(&db, &m, &b.id, None, "oi", &mut |e| ev.push(e), &now).unwrap();
        match ev.last() {
            Some(ChatEvent::Done { input_tokens, output_tokens, cost, .. }) => {
                assert_eq!((*input_tokens, *output_tokens), (20, 4));
                assert!(cost.unwrap() > 0.0); // modelo padrão está na tabela de preços
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn monthly_limit_blocks_new_answers() {
        let (_d, db, b) = setup("conselheira");
        {
            let g = db.lock().unwrap();
            let c = g.as_ref().unwrap();
            crate::db::settings::setting_set(c, "ai.monthly_limit", "1,50").unwrap();
            ai::usage_add(c, &month_of(now()), "x", 0, 0, Some(1.6)).unwrap();
        }
        let m = FakeModel::new(vec![text_turn("não devia")]);
        let mut ev = vec![];
        assert!(send(&db, &m, &b.id, None, "oi", &mut |e| ev.push(e), &now).is_err());
        assert!(matches!(ev.last(), Some(ChatEvent::Error { message }) if message.contains("limite")));
        assert!(m.calls.lock().unwrap().is_empty());
        assert_eq!(msgs(&db, &b.id), vec![("user".into(), "oi".into())]);
    }

    #[test]
    fn retry_does_not_duplicate_user_message() {
        let (_d, db, b) = setup("conselheira");
        let m = FakeModel::new(vec![Err(ProviderError::Network("x".into())), text_turn("Agora sim")]);
        let _ = send(&db, &m, &b.id, None, "oi", &mut |_| {}, &now);
        retry(&db, &m, &b.id, None, &mut |_| {}, &now).unwrap();
        assert_eq!(msgs(&db, &b.id), vec![("user".into(), "oi".into()), ("assistant".into(), "Agora sim".into())]);
    }

    #[test]
    fn coautora_proposal_emits_proposal_event() {
        let (_d, db, b) = setup("coautora");
        let m = FakeModel::new(vec![tool_turn("propor_trecho", serde_json::json!({"texto":"Rafa sorriu."})), text_turn("Propus um trecho.")]);
        let mut ev = vec![];
        send(&db, &m, &b.id, Some(&b.scenes[0]), "escreve algo", &mut |e| ev.push(e), &now).unwrap();
        assert!(ev.iter().any(|e| matches!(e, ChatEvent::Proposal { proposal } if proposal.text == "Rafa sorriu.")));
    }

    #[test]
    fn usage_accumulates_per_call() {
        let (_d, db, b) = setup("conselheira");
        let m = FakeModel::new(vec![tool_turn("buscar_no_livro", serde_json::json!({"texto":"Rafa"})), text_turn("ok")]);
        send(&db, &m, &b.id, None, "oi", &mut |_| {}, &now).unwrap();
        let g = db.lock().unwrap();
        let u = ai::usage_month(g.as_ref().unwrap(), "2026-11").unwrap();
        assert_eq!(u[0].input_tokens, 20);
    }

    #[test]
    fn fold_moves_old_messages_into_summary() {
        let (_d, db, b) = setup("conselheira");
        {
            let g = db.lock().unwrap();
            let c = g.as_ref().unwrap();
            let cv = ai::conversation_current(c, &b.id, 0).unwrap();
            for i in 0..30 {
                ai::message_add(c, &cv.id, if i % 2 == 0 { "user" } else { "assistant" }, &format!("m{i}"), None, i).unwrap();
            }
        }
        let m = FakeModel::new(vec![text_turn("Resumo: falamos de m0 a m17.")]);
        fold_if_needed(&db, &m, &b.id, &now).unwrap();
        let g = db.lock().unwrap();
        let cv = ai::conversation_current(g.as_ref().unwrap(), &b.id, 0).unwrap();
        assert_eq!(cv.folded_count, 18);
        assert!(cv.rolling_summary.contains("m0 a m17"));
        assert_eq!(m.calls.lock().unwrap()[0].0, DEFAULT_BACKGROUND_MODEL);
        drop(g);
        fold_if_needed(&db, &m, &b.id, &now).unwrap();
        assert_eq!(m.n_calls(), 1);
    }

    #[test]
    fn send_saves_question_then_folds_then_answers() {
        let (_d, db, b) = setup("conselheira");
        {
            let g = db.lock().unwrap();
            let c = g.as_ref().unwrap();
            let cv = ai::conversation_current(c, &b.id, 0).unwrap();
            for i in 0..30 {
                ai::message_add(c, &cv.id, if i % 2 == 0 { "user" } else { "assistant" }, &format!("m{i}"), None, i).unwrap();
            }
        }
        // o resumo falha (rede): a pergunta já tem que estar salva e a resposta segue
        let m = FakeModel::new(vec![Err(ProviderError::Network("x".into())), text_turn("ok")]);
        send(&db, &m, &b.id, None, "pergunta nova", &mut |_| {}, &now).unwrap();
        let calls = m.calls.lock().unwrap();
        assert_eq!(calls[0].0, DEFAULT_BACKGROUND_MODEL);
        assert_eq!(calls[1].0, DEFAULT_CHAT_MODEL);
        let all = msgs(&db, &b.id);
        assert_eq!(all[all.len() - 2], ("user".into(), "pergunta nova".into()));
    }

    #[test]
    fn month_of_epoch_ms() {
        assert_eq!(month_of(1_794_900_000_000), "2026-11");
        assert_eq!(month_of(0), "1970-01");
    }

    #[test]
    #[ignore]
    fn live_chat_and_background() {
        let key = std::env::var("INWORLD_KEY").expect("INWORLD_KEY");
        let model = crate::ai::provider::Inworld { key };
        let (_d, db, b) = setup("conselheira");
        let real_now = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
        for s in &b.scenes {
            crate::ai::background::digest_scene(&db, &model, s, &real_now).unwrap();
        }
        let mut ev = vec![];
        send(&db, &model, &b.id, Some(&b.scenes[0]), "Em que cena o Rafa descobre a verdade sobre o código?", &mut |e| ev.push(e), &real_now).unwrap();
        let answer = msgs(&db, &b.id).into_iter().filter(|(r, _)| r == "assistant").last().unwrap().1;
        let g = db.lock().unwrap();
        let c = g.as_ref().unwrap();
        let bible: Vec<_> = ai::bible_list(c, &b.id).unwrap().into_iter().map(|e| format!("{} ({}): {}", e.name, e.kind, e.body)).collect();
        let usage = ai::usage_month(c, &month_of(real_now())).unwrap();
        eprintln!("BIBLIA: {bible:#?}\nRESPOSTA: {answer}\nNOTAS: {:?}\nUSO: {usage:?}",
            ev.iter().filter_map(|e| if let ChatEvent::Note { text } = e { Some(text.clone()) } else { None }).collect::<Vec<_>>());
        assert!(answer.contains("Elevador"));
        assert!(!bible.is_empty());
    }

    #[test]
    #[ignore]
    fn live_parallel_tool_calls() {
        let key = std::env::var("INWORLD_KEY").expect("INWORLD_KEY");
        let model = crate::ai::provider::Inworld { key };
        let (_d, db, b) = setup("conselheira");
        let real_now = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
        let mut ev = vec![];
        send(&db, &model, &b.id, None, "Leia as três cenas inteiras, todas ao mesmo tempo (chamadas em paralelo), e me diga em uma frase o que acontece em cada.", &mut |e| ev.push(e), &real_now).unwrap();
        let notes = ev.iter().filter(|e| matches!(e, ChatEvent::Note { .. })).count();
        eprintln!("notas: {notes}");
        assert!(notes >= 2);
        assert!(matches!(ev.last(), Some(ChatEvent::Done { .. })));
    }
}
