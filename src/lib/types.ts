export type VaultStatus = "none" | "locked" | "unlocked";
export type AiMode = "conselheira" | "coautora";

/** Códigos de erro vindos do Rust. */
export type ErrorCode =
  | "WRONG_PASSWORD"
  | "INVALID_PHRASE"
  | "NO_VAULT"
  | "VAULT_EXISTS"
  | "LOCKED"
  | "CHAPTER_NOT_EMPTY";

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
