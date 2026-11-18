// Só em desenvolvimento, fora do Tauri (navegador comum): backend em memória para revisar a interface.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

type Row = Record<string, any>;
const db = { books: [] as Row[], chapters: [] as Row[], scenes: [] as Row[], versions: [] as Row[], settings: {} as Record<string, string> };
let status: "none" | "locked" | "unlocked" = "none";
const id = () => crypto.randomUUID();
const EMPTY = '{"type":"doc","content":[{"type":"paragraph"}]}';
const ordered = (rows: Row[]) => rows.sort((a, b) => a.position - b.position);
const renumber = (rows: Row[]) => ordered(rows).forEach((r, i) => (r.position = i));
const place = (list: Row[], row: Row, index?: number | null) => {
  const l = ordered(list.filter((r) => r !== row));
  l.splice(index == null ? l.length : Math.max(0, Math.min(index, l.length)), 0, row);
  l.forEach((r, i) => (r.position = i));
};
const scenesOf = (ch: string) => db.scenes.filter((s) => s.chapter_id === ch);
const chaptersOf = (b: string) => db.chapters.filter((c) => c.book_id === b);

const handlers: Record<string, (a: any) => any> = {
  vault_status: () => status,
  vault_create: () => ((status = "unlocked"), "abacate abaixo abalar abater abduzir abelha aberto abismo abotoar abranger abreviar abrigar abrupto absinto absoluto absurdo abutre acabado acalmar acampar acanhar acaso aceitar acelerar"),
  vault_unlock: ({ password }) => { if (password !== "senha123") throw "WRONG_PASSWORD"; status = "unlocked"; },
  vault_recover: () => { status = "unlocked"; },
  vault_lock: () => { status = "locked"; },
  vault_nickname: () => (status === "none" ? null : "henrilgrim"),
  book_list: () => db.books,
  book_create: ({ title, aiMode }) => {
    const b = { id: id(), title, ai_mode: aiMode, created_at: Date.now() };
    db.books.push(b);
    const ch = handlers.chapter_create({ bookId: b.id, title: "Capítulo 1" });
    handlers.scene_create({ chapterId: ch.id, title: "Cena 1" });
    return b;
  },
  book_outline: ({ bookId }) => ({
    book: db.books.find((b) => b.id === bookId),
    chapters: ordered(chaptersOf(bookId)).map((chapter) => ({
      chapter,
      scenes: ordered(scenesOf(chapter.id)).map(({ content_json, updated_at, ...s }) => s),
    })),
  }),
  chapter_create: ({ bookId, title, index }) => {
    const c = { id: id(), book_id: bookId, title, position: 0 };
    db.chapters.push(c);
    place(chaptersOf(bookId), c, index);
    return c;
  },
  chapter_rename: ({ id, title }) => { db.chapters.find((c) => c.id === id)!.title = title; },
  chapter_delete: ({ id }) => {
    if (scenesOf(id).length) throw "CHAPTER_NOT_EMPTY";
    const c = db.chapters.find((x) => x.id === id)!;
    db.chapters = db.chapters.filter((x) => x !== c);
    renumber(chaptersOf(c.book_id));
  },
  scene_create: ({ chapterId, title, index }) => {
    const s = { id: id(), chapter_id: chapterId, title, content_json: EMPTY, word_count: 0, position: 0, updated_at: Date.now() };
    db.scenes.push(s);
    place(scenesOf(chapterId), s, index);
    return s;
  },
  scene_get: ({ id }) => db.scenes.find((s) => s.id === id),
  scene_rename: ({ id, title }) => { db.scenes.find((s) => s.id === id)!.title = title; },
  scene_save: ({ id, contentJson, wordCount }) => {
    const s = db.scenes.find((x) => x.id === id)!;
    Object.assign(s, { content_json: contentJson, word_count: wordCount, updated_at: Date.now() });
    const mine = db.versions.filter((v) => v.scene_id === id); const last = mine[mine.length - 1];
    if (!last || Date.now() - last.created_at > 600_000)
      db.versions.push({ id: crypto.randomUUID(), scene_id: id, content_json: contentJson, word_count: wordCount, created_at: Date.now() });
  },
  scene_move: ({ id, toChapterId, toIndex }) => {
    const s = db.scenes.find((x) => x.id === id)!;
    const from = s.chapter_id;
    s.chapter_id = toChapterId;
    place(scenesOf(toChapterId), s, toIndex);
    renumber(scenesOf(from));
  },
  scene_delete: ({ id }) => {
    const s = db.scenes.find((x) => x.id === id)!;
    db.scenes = db.scenes.filter((x) => x !== s);
    renumber(scenesOf(s.chapter_id));
  },
  scene_versions: ({ sceneId }) =>
    db.versions.filter((v) => v.scene_id === sceneId).reverse().map(({ id, created_at, word_count }) => ({ id, created_at, word_count })),
  scene_restore: ({ versionId }) => {
    const v = db.versions.find((x) => x.id === versionId)!;
    const s = db.scenes.find((x) => x.id === v.scene_id)!;
    Object.assign(s, { content_json: v.content_json, word_count: v.word_count });
    return s;
  },
  setting_get: ({ key }) => db.settings[key] ?? null,
  setting_set: ({ key, value }) => { db.settings[key] = value; },
};

let mockRead = false; // simula 3 cenas a ler até a primeira leitura

// celular e caixa de entrada (simulados)
let remote = { connected: false, nickname: "henrilgrim", url: "https://lume.codexolimpo.com" };
let inbox: Row[] = [];
Object.assign(handlers, {
  remote_status: () => ({ connected: remote.connected, nickname: remote.connected ? remote.nickname : null, url: remote.connected ? remote.url : null }),
  remote_connect: ({ password }: any) => {
    if (password !== "senha123") throw "Apelido ou senha incorretos";
    remote.connected = true;
    inbox = [
      { id: "i1", kind: "text", mime: "text/plain", size: 80, created_at: Date.now() - 3600_000, status: "nova", text: "E se a Marina encontrar a carta antes do casamento?\nO irmão sabia de tudo." },
      { id: "i2", kind: "audio", mime: "audio/webm", size: 2000, created_at: Date.now() - 600_000, status: "nova", text: null, transcript: null, transcript_status: "pendente", transcript_error: null },
      { id: "i3", kind: "audio", mime: "audio/mp4", size: 900, created_at: Date.now() - 300_000, status: "nova", text: null, transcript: null, transcript_status: "erro", transcript_error: "Sem crédito ou limite atingido na Inworld" },
    ];
  },
  remote_disconnect: () => void (remote.connected = false),
  remote_sync: () => 0,
  inbox_list: () => [...inbox].sort((a, b) => b.created_at - a.created_at),
  inbox_body: () => new ArrayBuffer(0),
  inbox_transcribe_pending: () => {
    let n = 0;
    for (const i of inbox)
      if (i.kind === "audio" && i.transcript_status === "pendente") {
        Object.assign(i, { transcript_status: "pronta", transcript: "Texto transcrito de exemplo: e se o Teodoro já soubesse da carta desde o começo?" });
        n++;
      }
    return n;
  },
  inbox_retry_transcript: ({ id }: any) => void Object.assign(inbox.find((i) => i.id === id)!, { transcript_status: "pendente", transcript_error: null }),
  inbox_archive: ({ id }: any) => void (inbox.find((i) => i.id === id)!.status = "arquivada"),
  inbox_restore: ({ id }: any) => void (inbox.find((i) => i.id === id)!.status = "nova"),
  inbox_delete: ({ id }: any) => void (inbox = inbox.filter((i) => i.id !== id)),
  inbox_to_scene: ({ id, chapterId, index }: any) => {
    const it = inbox.find((i) => i.id === id)!;
    const source = it.kind === "text" ? it.text : it.transcript_status === "pronta" ? it.transcript : null;
    if (!source?.trim()) throw "NOT_TEXT";
    const lines = source.split("\n").filter(Boolean);
    const s = handlers.scene_create({ chapterId, title: lines[0].slice(0, 60), index });
    s.content_json = JSON.stringify({ type: "doc", content: lines.map((t: string) => ({ type: "paragraph", content: [{ type: "text", text: t }] })) });
    it.status = "arquivada";
    return s;
  },
});

// assistente de IA (simulado): responde devagar, consulta uma cena e mantém fichas
const ai = { key: false, conv: [] as Row[], msgs: [] as Row[], bible: [] as Row[], models: { chat: "anthropic/claude-sonnet-5-5", background: "anthropic/claude-haiku-5-5" } };
const curConv = (bookId: string) => {
  let c = ai.conv.find((x) => x.book_id === bookId && !x.closed);
  if (!c) { c = { id: id(), book_id: bookId, title: "Conversa", rolling_summary: "", folded_count: 0, closed: false, created_at: Date.now(), updated_at: Date.now() }; ai.conv.push(c); }
  return c;
};
const view = (c: Row) => ({ conversation: c, messages: ai.msgs.filter((m) => m.conversation_id === c.id) });
const addMsg = (c: Row, role: string, content: string) => { const m = { id: id(), conversation_id: c.id, role, content, tool_name: null, created_at: Date.now() }; ai.msgs.push(m); return m; };
const send = (ch: any, e: any) => ch?.onmessage?.(e);
Object.assign(handlers, {
  ai_key_set: () => void (ai.key = true),
  ai_key_status: () => ai.key,
  ai_models_get: () => ai.models,
  ai_models_set: ({ chat, background }: any) => void (ai.models = { chat, background }),
  ai_usage: () => (ai.key ? [{ month: "2026-11", model: ai.models.chat, input_tokens: 48200, output_tokens: 3100, cost: 0.13 }] : []),
  ai_enable: () => undefined,
  ai_conversation: ({ bookId }: any) => view(curConv(bookId)),
  ai_conversation_new: ({ bookId }: any) => { ai.conv.forEach((c) => c.book_id === bookId && (c.closed = true)); return view(curConv(bookId)); },
  ai_conversations: ({ bookId }: any) => ai.conv.filter((c) => c.book_id === bookId),
  ai_conversation_rename: ({ id: cid, title }: any) => void (ai.conv.find((x) => x.id === cid)!.title = title),
  ai_conversation_delete: ({ id: cid }: any) => { ai.conv = ai.conv.filter((x) => x.id !== cid); ai.msgs = ai.msgs.filter((m) => m.conversation_id !== cid); },
  ai_conversation_open: ({ id: cid }: any) => { const c = ai.conv.find((x) => x.id === cid)!; ai.conv.forEach((x) => x.book_id === c.book_id && (x.closed = true)); c.closed = false; return view(c); },
  ai_send: async ({ bookId, text, onEvent }: any) => {
    const c = curConv(bookId);
    if (c.title === "Conversa") c.title = text.slice(0, 48);
    addMsg(c, "user", text);
    if (!ai.key) { send(onEvent, { type: "error", message: "Configure a chave da Inworld em Ajustes" }); throw "Configure a chave da Inworld em Ajustes"; }
    const note = "consultei a cena \"Linha 340\""; addMsg(c, "note", note); send(onEvent, { type: "note", text: note });
    const answer = "Pelo que li, o Rafa ainda não sabe do bug nessa cena — ele só descobre mais adiante, no elevador.";
    for (const w of answer.split(/(?<= )/)) { await new Promise((r) => setTimeout(r, 25)); send(onEvent, { type: "delta", text: w }); }
    const m = addMsg(c, "assistant", answer);
    if (!ai.bible.length) ai.bible.push({ id: id(), book_id: bookId, kind: "personagem", name: "Rafa", aliases: ["Rafael"], body: "Programador, escreve de madrugada.", ai_notes: [], story_order: null, source_scene_id: db.scenes[0]?.id ?? null, edited_by_user: false, updated_at: Date.now() });
    send(onEvent, { type: "done", message_id: m.id, input_tokens: 5200, output_tokens: 140, cost: 0.0118 });
  },
  ai_retry: () => undefined,
  ai_scene_touched: () => undefined,
  ai_first_pass: async ({ onProgress }: any) => { for (let i = 0; i <= 3; i++) { send(onProgress, { done: i, total: 3 }); await new Promise((r) => setTimeout(r, 300)); } mockRead = true; },
  ai_first_pass_cancel: () => undefined,
  ai_pending_count: () => (mockRead ? 0 : 3),
  bible_list: ({ bookId }: any) => ai.bible.filter((e) => e.book_id === bookId),
  bible_update: ({ id: eid, name, aliases, body, storyOrder }: any) => Object.assign(ai.bible.find((e) => e.id === eid)!, { name, aliases, body, story_order: storyOrder, edited_by_user: true }),
  bible_absorb_note: ({ id: eid, index }: any) => { const e = ai.bible.find((x) => x.id === eid)!; const [n] = e.ai_notes.splice(index, 1); e.body += "\n" + n.fact; return e; },
  bible_delete: ({ id: eid }: any) => void (ai.bible = ai.bible.filter((e) => e.id !== eid)),
  proposal_set_status: () => undefined,
  proposals_pending: () => [],
});

export function installMockBackend() {
  mockWindows("main");
  if (location.search.includes("demo")) {
    status = "unlocked";
    const b = handlers.book_create({ title: "Até ter certeza", aiMode: location.search.includes("conselheira") ? "conselheira" : "coautora" });
    db.chapters = []; db.scenes = [];
    ["Lugar de Sempre", "Linha 340", "A Porta do Elevador"].forEach((t) => {
      const ch = handlers.chapter_create({ bookId: b.id, title: t });
      const s = handlers.scene_create({ chapterId: ch.id, title: t });
      const text = "A tela do notebook é a única luz do quarto. Já são quase duas da manhã e eu ainda estou aqui.";
      s.content_json = JSON.stringify({ type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text }] }] });
      s.word_count = text.split(/\s+/).length;
    });
    ai.key = true;
    db.settings["assistant_open"] = "1";
  }
  mockIPC((cmd, args) => {
    const h = handlers[cmd];
    if (!h) return undefined; // eventos/janela: ignora
    return h(args ?? {});
  });
  console.info("[Lume] backend simulado em memória (senha do desbloqueio: senha123)");
}
