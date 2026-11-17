import type { JSONContent } from "@tiptap/core";

export type ExportBook = {
  title: string;
  chapters: { title: string; scenes: { title: string; doc: JSONContent }[] }[];
};
