export type VaultStatus = "none" | "locked" | "unlocked" | "missing_vault";
export type AiMode = "conselheira" | "coautora";

/** Códigos de erro vindos do Rust. */
export type ErrorCode =
  | "WRONG_PASSWORD"
  | "INVALID_PHRASE"
  | "NO_VAULT"
  | "VAULT_EXISTS"
  | "LOCKED"
  | "CHAPTER_NOT_EMPTY"
  | "DB_EXISTS"
  | "WRONG_KEY"
  | "NOT_FOUND"
  | "NOT_CONNECTED"
  | "NOT_AUTHENTICATED"
  | "KEY_MISMATCH"
  | "NOT_TEXT";

export interface Book { id: string; title: string; ai_mode: AiMode; created_at: number }
export interface Chapter { id: string; book_id: string; title: string; position: number }
export interface SceneSummary { id: string; chapter_id: string; title: string; word_count: number; position: number }
export interface ChapterOutline { chapter: Chapter; scenes: SceneSummary[] }
export interface Outline { book: Book; chapters: ChapterOutline[] }
export interface Scene {
  id: string; chapter_id: string; title: string; content_json: string;
  word_count: number; position: number; updated_at: number;
}
export interface Version { id: string; created_at: number; word_count: number }

export interface RemoteStatus { connected: boolean; nickname: string | null; url: string | null }
export interface InboxItem {
  id: string; kind: "audio" | "text"; mime: string; size: number; created_at: number;
  status: "nova" | "arquivada"; text: string | null;
  transcript: string | null;
  transcript_status: "pendente" | "pronta" | "erro" | "sem_chave" | null;
  transcript_error: string | null;
}

// ---------- assistente de IA ----------
export type BibleKind = "personagem" | "lugar" | "fato" | "linha_do_tempo";
export interface AiNote { fact: string; source_scene_id: string | null }
export interface BibleEntry {
  id: string; book_id: string; kind: BibleKind; name: string; aliases: string[]; body: string;
  ai_notes: AiNote[]; story_order: number | null; source_scene_id: string | null; edited_by_user: boolean; updated_at: number;
}
export interface Conversation {
  id: string; book_id: string; title: string; rolling_summary: string; folded_count: number;
  closed: boolean; created_at: number; updated_at: number;
}
export interface AiMessage { id: string; conversation_id: string; role: "user" | "assistant" | "note"; content: string; tool_name: string | null; created_at: number }
export interface ConversationView { conversation: Conversation; messages: AiMessage[] }
export interface Proposal {
  id: string; book_id: string; conversation_id: string | null; scene_id: string | null; text: string;
  status: "pendente" | "inserida" | "virou_cena" | "descartada"; created_at: number;
}
export interface UsageRow { month: string; model: string; input_tokens: number; output_tokens: number; cost: number | null }
export interface AiModels { chat: string; background: string }
export type ChatEvent =
  | { type: "delta"; text: string }
  | { type: "note"; text: string }
  | { type: "proposal"; proposal: Proposal }
  | { type: "done"; message_id: string; input_tokens: number; output_tokens: number; cost: number | null }
  | { type: "error"; message: string };
export interface Progress { done: number; total: number }
