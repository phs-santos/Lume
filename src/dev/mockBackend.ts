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
  vault_create: () => ((status = "unlocked"), "abandon ability able about above absent absorb abstract absurd abuse access accident account accuse achieve acid acoustic acquire across act action actor actress actual"),
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
      { id: "i2", kind: "audio", mime: "audio/webm", size: 2000, created_at: Date.now() - 600_000, status: "nova", text: null },
    ];
  },
  remote_disconnect: () => void (remote.connected = false),
  remote_sync: () => 0,
  inbox_list: () => [...inbox].sort((a, b) => b.created_at - a.created_at),
  inbox_body: () => new ArrayBuffer(0),
  inbox_archive: ({ id }: any) => void (inbox.find((i) => i.id === id)!.status = "arquivada"),
  inbox_restore: ({ id }: any) => void (inbox.find((i) => i.id === id)!.status = "nova"),
  inbox_delete: ({ id }: any) => void (inbox = inbox.filter((i) => i.id !== id)),
  inbox_to_scene: ({ id, chapterId, index }: any) => {
    const it = inbox.find((i) => i.id === id)!;
    if (it.kind !== "text") throw "NOT_TEXT";
    const lines = it.text.split("\n").filter(Boolean);
    const s = handlers.scene_create({ chapterId, title: lines[0].slice(0, 60), index });
    s.content_json = JSON.stringify({ type: "doc", content: lines.map((t: string) => ({ type: "paragraph", content: [{ type: "text", text: t }] })) });
    it.status = "arquivada";
    return s;
  },
});

export function installMockBackend() {
  mockWindows("main");
  mockIPC((cmd, args) => {
    const h = handlers[cmd];
    if (!h) return undefined; // eventos/janela: ignora
    return h(args ?? {});
  });
  console.info("[Lume] backend simulado em memória (senha do desbloqueio: senha123)");
}
