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
}
