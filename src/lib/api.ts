import { invoke } from "@tauri-apps/api/core";
import type { VaultStatus } from "./types";

export const vaultStatus = () => invoke<VaultStatus>("vault_status");
export const vaultCreate = (nickname: string, password: string) =>
  invoke<string>("vault_create", { nickname, password });
export const vaultUnlock = (password: string) => invoke<void>("vault_unlock", { password });
export const vaultRecover = (phrase: string, newPassword: string) =>
  invoke<void>("vault_recover", { phrase, newPassword });
export const vaultLock = () => invoke<void>("vault_lock");
