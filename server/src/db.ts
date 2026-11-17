import { DatabaseSync } from "node:sqlite";

const MIGRATIONS = [
  `CREATE TABLE accounts(
     id TEXT PRIMARY KEY,
     nickname TEXT NOT NULL UNIQUE COLLATE NOCASE,
     salt BLOB NOT NULL,
     secret_hash TEXT NOT NULL,
     public_key BLOB,
     created_at INTEGER NOT NULL
   );
   CREATE TABLE sessions(
     token_hash TEXT PRIMARY KEY,
     account_id TEXT NOT NULL REFERENCES accounts(id),
     created_at INTEGER NOT NULL,
     expires_at INTEGER NOT NULL
   );
   CREATE TABLE envelopes(
     id TEXT PRIMARY KEY,
     account_id TEXT NOT NULL REFERENCES accounts(id),
     kind TEXT NOT NULL,
     mime TEXT NOT NULL,
     body BLOB NOT NULL,
     created_at INTEGER NOT NULL
   );
   CREATE INDEX envelopes_account ON envelopes(account_id, created_at);`,
];

export function openDb(path: string): DatabaseSync {
  const db = new DatabaseSync(path);
  db.exec("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;");
  const { user_version } = db.prepare("PRAGMA user_version").get() as { user_version: number };
  MIGRATIONS.slice(user_version).forEach((sql, i) => {
    db.exec(`BEGIN; ${sql}; PRAGMA user_version = ${user_version + i + 1}; COMMIT;`);
  });
  return db;
}
