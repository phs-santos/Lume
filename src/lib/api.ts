import { invoke } from "@tauri-apps/api/core";
import type { AiMode, Book, InboxItem, RemoteStatus, Chapter, Outline, Scene, Version, VaultStatus } from "./types";

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
export const inboxBody = (id: string) => invoke<number[]>("inbox_body", { id });
export const inboxArchive = (id: string) => invoke<void>("inbox_archive", { id });
export const inboxRestore = (id: string) => invoke<void>("inbox_restore", { id });
export const inboxDelete = (id: string) => invoke<void>("inbox_delete", { id });
export const inboxToScene = (id: string, chapterId: string, index?: number) =>
  invoke<Scene>("inbox_to_scene", { id, chapterId, index: index ?? null });
