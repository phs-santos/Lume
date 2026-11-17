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
