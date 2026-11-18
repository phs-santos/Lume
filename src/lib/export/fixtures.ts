import type { ExportBook } from "./types";

const p = (...content: any[]) => ({ type: "paragraph", content });
const t = (text: string, marks?: string[]) => ({ type: "text", text, ...(marks ? { marks: marks.map((type) => ({ type })) } : {}) });
export const doc = (...content: any[]) => ({ type: "doc", content });

export const book: ExportBook = {
  title: "A Carta",
  chapters: [
    {
      title: "Capítulo 1",
      scenes: [
        { title: "Cena 1", doc: doc(p(t("— Você viu? *Não*, disse ela. #1 e 2_3")), p(t("Ela riu "), t("baixinho", ["italic"]), t(" e "), t("saiu", ["bold"]), t("."))) },
        { title: "Cena 2", doc: doc(p(t("Depois."))) },
      ],
    },
    { title: "Capítulo vazio", scenes: [] },
  ],
};

/** Livro com imagens no texto (com e sem legenda). */
export const withFigures: ExportBook = {
  title: "Com imagens",
  chapters: [
    {
      title: "Capítulo 1",
      scenes: [{ title: "Cena", doc: doc(p(t("Antes.")), { type: "figure", attrs: { src: "lume-img:a", caption: "Mapa da serra" } }, { type: "figure", attrs: { src: "lume-img:b", caption: "" } }, p(t("Depois."))) }],
    },
  ],
};
