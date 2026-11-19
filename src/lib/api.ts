import { Channel, invoke } from "@tauri-apps/api/core";
import type { AiModels, BibleEntry, CardColor, ChatEvent, Conversation, ConversationView, MapCard, MapData, MapLink, Progress, Proposal, UsageRow } from "./types";
import type { AiMode, Book, BookMeta, InboxItem, RemoteStatus, Chapter, Outline, Scene, Version, VaultStatus } from "./types";

export interface Profile { id: string; nickname: string; active: boolean }
export const profilesList = () => invoke<Profile[]>("profiles_list");
export const profileSelect = (id: string) => invoke<void>("profile_select", { id });
export const profileNew = () => invoke<string>("profile_new");
export const vaultStatus = () => invoke<VaultStatus>("vault_status");
export const vaultCreate = (nickname: string, password: string) =>
  invoke<string>("vault_create", { nickname, password });
export const vaultUnlock = (password: string) => invoke<void>("vault_unlock", { password });
export const vaultRecover = (phrase: string, newPassword: string) =>
  invoke<void>("vault_recover", { phrase, newPassword });
export const vaultLock = () => invoke<void>("vault_lock");
export const vaultNickname = () => invoke<string | null>("vault_nickname");

export const bookList = () => invoke<Book[]>("book_list");
export const bookCreate = (title: string, aiMode: AiMode) => invoke<Book>("book_create", { title, aiMode });
export const bookRename = (id: string, title: string) => invoke<void>("book_rename", { id, title });
export const bookSetMode = (id: string, aiMode: AiMode) => invoke<void>("book_set_mode", { id, aiMode });
export const bookImportEpub = (path: string, aiMode: AiMode) => invoke<Book>("book_import_epub", { path, aiMode });
export const bookOutline = (bookId: string) => invoke<Outline>("book_outline", { bookId });

export const chapterCreate = (bookId: string, title: string, index?: number) =>
  invoke<Chapter>("chapter_create", { bookId, title, index: index ?? null });
export const chapterRename = (id: string, title: string) => invoke<void>("chapter_rename", { id, title });
export const chapterMove = (id: string, toIndex: number) => invoke<void>("chapter_move", { id, toIndex });
export const chapterDelete = (id: string) => invoke<void>("chapter_delete", { id });

export const sceneCreate = (chapterId: string, title: string, index?: number) =>
  invoke<Scene>("scene_create", { chapterId, title, index: index ?? null });
export const sceneGet = (id: string) => invoke<Scene>("scene_get", { id });
export const sceneRename = (id: string, title: string) => invoke<void>("scene_rename", { id, title });
export const sceneSave = (id: string, contentJson: string, wordCount: number) =>
  invoke<void>("scene_save", { id, contentJson, wordCount });
export const sceneMove = (id: string, toChapterId: string, toIndex: number) =>
  invoke<void>("scene_move", { id, toChapterId, toIndex });
export const sceneDelete = (id: string) => invoke<void>("scene_delete", { id });
export const sceneVersions = (sceneId: string) => invoke<Version[]>("scene_versions", { sceneId });
export const sceneRestore = (versionId: string) => invoke<Scene>("scene_restore", { versionId });

export const settingGet = (key: string) => invoke<string | null>("setting_get", { key });
export const settingSet = (key: string, value: string) => invoke<void>("setting_set", { key, value });

export const remoteStatus = () => invoke<RemoteStatus>("remote_status");
export const remoteConnect = (url: string, nickname: string, password: string, invite?: string) =>
  invoke<void>("remote_connect", { url, nickname, password, invite: invite || null });
export const remoteDisconnect = () => invoke<void>("remote_disconnect");
export const remoteSync = () => invoke<number>("remote_sync");
export const inboxList = () => invoke<InboxItem[]>("inbox_list");
export const inboxBody = (id: string) => invoke<ArrayBuffer>("inbox_body", { id });
export const inboxArchive = (id: string) => invoke<void>("inbox_archive", { id });
export const inboxRestore = (id: string) => invoke<void>("inbox_restore", { id });
export const inboxDelete = (id: string) => invoke<void>("inbox_delete", { id });
export const inboxTranscribePending = () => invoke<number>("inbox_transcribe_pending");
export const inboxRetryTranscript = (id: string) => invoke<void>("inbox_retry_transcript", { id });
export const inboxToScene = (id: string, chapterId: string, index?: number) =>
  invoke<Scene>("inbox_to_scene", { id, chapterId, index: index ?? null });

// ---------- assistente de IA (a chave nunca volta para cá) ----------
export const aiKeySet = (key: string) => invoke<void>("ai_key_set", { key });
export const aiKeyStatus = () => invoke<boolean>("ai_key_status");
export const aiModelsGet = () => invoke<AiModels>("ai_models_get");
export const aiModelsSet = (chat: string, background: string) => invoke<void>("ai_models_set", { chat, background });
export const aiUsage = (month: string) => invoke<UsageRow[]>("ai_usage", { month });
export const aiEnable = (bookId: string) => invoke<void>("ai_enable", { bookId });
export const aiConversation = (bookId: string) => invoke<ConversationView>("ai_conversation", { bookId });
export const aiConversationNew = (bookId: string) => invoke<ConversationView>("ai_conversation_new", { bookId });
export const aiConversations = (bookId: string) => invoke<Conversation[]>("ai_conversations", { bookId });
export const aiConversationOpen = (id: string) => invoke<ConversationView>("ai_conversation_open", { id });
export const aiConversationRename = (id: string, title: string) => invoke<void>("ai_conversation_rename", { id, title });
export const aiConversationDelete = (id: string) => invoke<void>("ai_conversation_delete", { id });

function channel<T>(on: (e: T) => void) {
  const ch = new Channel<T>();
  ch.onmessage = on;
  return ch;
}
export const aiSend = (bookId: string, sceneId: string | null, text: string, on: (e: ChatEvent) => void) =>
  invoke<void>("ai_send", { bookId, sceneId, text, onEvent: channel(on) });
export const aiRetry = (bookId: string, sceneId: string | null, on: (e: ChatEvent) => void) =>
  invoke<void>("ai_retry", { bookId, sceneId, onEvent: channel(on) });
export const aiSceneTouched = (sceneId: string) => invoke<void>("ai_scene_touched", { sceneId });
export const aiFirstPass = (bookId: string, on: (p: Progress) => void) => invoke<void>("ai_first_pass", { bookId, onProgress: channel(on) });
export const aiFirstPassCancel = () => invoke<void>("ai_first_pass_cancel");
export const aiPendingCount = (bookId: string) => invoke<number>("ai_pending_count", { bookId });

export const bibleList = (bookId: string) => invoke<BibleEntry[]>("bible_list", { bookId });
export const bibleUpdate = (id: string, name: string, aliases: string[], body: string, storyOrder: number | null) =>
  invoke<BibleEntry>("bible_update", { id, name, aliases, body, storyOrder });
export const bibleAbsorbNote = (id: string, index: number) => invoke<BibleEntry>("bible_absorb_note", { id, index });
export const bibleDelete = (id: string) => invoke<void>("bible_delete", { id });
export const proposalSetStatus = (id: string, status: Proposal["status"]) => invoke<void>("proposal_set_status", { id, status });
export const proposalsPending = (bookId: string) => invoke<Proposal[]>("proposals_pending", { bookId });
export const aiConversationNewIdea = (bookId: string) => invoke<ConversationView>("ai_conversation_new_idea", { bookId });

// ---------- mapa de ideias ----------
export const mapGet = (bookId: string) => invoke<MapData>("map_get", { bookId });
export const mapCardCreate = (bookId: string, text: string, color: CardColor, x: number, y: number) =>
  invoke<MapCard>("map_card_create", { bookId, text, color, x, y });
export const mapCardUpdate = (id: string, text: string, color: CardColor) => invoke<MapCard>("map_card_update", { id, text, color });
export const mapCardMove = (id: string, x: number, y: number) => invoke<void>("map_card_move", { id, x, y });
export const mapCardDelete = (id: string) => invoke<void>("map_card_delete", { id });
export const mapLinkCreate = (bookId: string, from: string, to: string) => invoke<MapLink>("map_link_create", { bookId, from, to });
export const mapLinkLabel = (id: string, label: string) => invoke<void>("map_link_label", { id, label });
export const mapLinkDelete = (id: string) => invoke<void>("map_link_delete", { id });
export const mapCardToBible = (id: string, kind: BibleEntry["kind"]) => invoke<BibleEntry>("map_card_to_bible", { id, kind });

export const imageAdd = (bookId: string, mime: string, b64: string, width: number, height: number) =>
  invoke<string>("image_add", { bookId, mime, b64, width, height });
export const imageGet = (id: string) => invoke<ArrayBuffer>("image_get", { id });
export const imageMime = (id: string) => invoke<string>("image_mime", { id });
export const imagesGc = (bookId: string) => invoke<number>("images_gc", { bookId });
export const bookMetaGet = (bookId: string) => invoke<BookMeta>("book_meta_get", { bookId });
export const bookMetaSet = (bookId: string, meta: BookMeta) => invoke<void>("book_meta_set", { bookId, meta });
