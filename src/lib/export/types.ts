import type { JSONContent } from "@tiptap/core";

export type ExportBook = {
  title: string;
  chapters: { title: string; scenes: { title: string; doc: JSONContent }[] }[];
};

export type ExportOptions = {
  /** Põe o título de cada cena (quando é diferente do título do capítulo). */
  sceneTitles?: boolean;
};
