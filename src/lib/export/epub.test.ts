// @vitest-environment happy-dom
import JSZip from "jszip";
import { describe, expect, it } from "vitest";
import { toEpub, type EpubInput } from "./epub";

const p = (t: string, marks?: string[]) => ({ type: "paragraph", content: [{ type: "text", text: t, ...(marks ? { marks: marks.map((type) => ({ type })) } : {}) }] });
const png = { mime: "image/png", bytes: new Uint8Array([137, 80, 78, 71]) };
const jpg = { mime: "image/jpeg", bytes: new Uint8Array([255, 216, 255]) };

const full: EpubInput = {
  id: "11111111-2222-3333-4444-555555555555",
  title: "Até ter certeza & <outros> \"contos\"",
  author: "Hefesto",
  synopsis: "João fica milionário em segredo.\nE agora?",
  dedication: "Para a Marina",
  cover: png,
  back: jpg,
  images: new Map([["img-a", jpg]]),
  chapters: [
    {
      title: "Linha 340",
      scenes: [
        { title: "Linha 340", doc: { type: "doc", content: [p("A tela é a única luz."), p("Ele ri", ["italic"]), { type: "figure", attrs: { src: "lume-img:img-a", caption: "Mapa & serra" } }] } },
        { title: "Depois", doc: { type: "doc", content: [p("Segunda cena.")] } },
      ],
    },
    { title: "A Porta do Elevador", scenes: [{ title: "x", doc: { type: "doc", content: [p("Fim.")] } }] },
  ],
};

async function open(b: EpubInput) {
  const bytes = await toEpub(b);
  const zip = await JSZip.loadAsync(bytes);
  const read = (f: string) => zip.file(f)!.async("string");
  // sem a <link> da folha de estilos: o happy-dom tentaria baixá-la depois do fim do teste
  const xml = (s: string) => new DOMParser().parseFromString(s.replace(/<link [^>]*\/>/g, ""), "application/xml");
  return { bytes, zip, read, xml };
}

describe("EPUB", () => {
  it("mimetype é a primeira entrada, sem compressão", async () => {
    const { bytes } = await open(full);
    // cabeçalho local do zip: nome em 30.., método de compressão nos bytes 8-9 (0 = STORE)
    expect(new TextDecoder().decode(bytes.slice(30, 38))).toBe("mimetype");
    expect(bytes[8] | (bytes[9] << 8)).toBe(0);
    expect(new TextDecoder().decode(bytes.slice(38, 58))).toBe("application/epub+zip");
  });

  it("metadados: título, autor, idioma, identificador estável e data", async () => {
    const { read, xml } = await open(full);
    const opf = xml(await read("OEBPS/content.opf"));
    const get = (tag: string) => opf.getElementsByTagName(tag)[0]?.textContent;
    expect(get("dc:title")).toBe("Até ter certeza & <outros> \"contos\"");
    expect(get("dc:creator")).toBe("Hefesto");
    expect(get("dc:language")).toBe("pt-BR");
    expect(get("dc:identifier")).toBe("urn:uuid:11111111-2222-3333-4444-555555555555");
    // o happy-dom trata <meta> como tag vazia do HTML; confere no texto cru
    expect(await read("OEBPS/content.opf")).toMatch(/<meta property="dcterms:modified">\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z<\/meta>/);
  });

  it("capa marcada como cover-image e todas as imagens no manifest", async () => {
    const { read, xml, zip } = await open(full);
    const items = [...xml(await read("OEBPS/content.opf")).getElementsByTagName("item")];
    const cover = items.find((i) => i.getAttribute("properties") === "cover-image")!;
    expect(cover.getAttribute("media-type")).toBe("image/png");
    expect(zip.file(`OEBPS/${cover.getAttribute("href")}`)).not.toBeNull();
    const imgs = items.filter((i) => i.getAttribute("media-type")?.startsWith("image/")).map((i) => i.getAttribute("href"));
    expect(imgs).toHaveLength(3); // capa, contracapa e a imagem do texto
    for (const h of imgs) expect(zip.file(`OEBPS/${h}`)).not.toBeNull();
  });

  it("ordem: capa, rosto, dedicatória, sumário, capítulos, contracapa", async () => {
    const { read, xml } = await open(full);
    const opf = xml(await read("OEBPS/content.opf"));
    const byId = new Map([...opf.getElementsByTagName("item")].map((i) => [i.getAttribute("id"), i.getAttribute("href")]));
    const order = [...opf.getElementsByTagName("itemref")].map((r) => byId.get(r.getAttribute("idref")));
    expect(order).toEqual(["cover.xhtml", "title.xhtml", "dedication.xhtml", "nav.xhtml", "chapter-1.xhtml", "chapter-2.xhtml", "back.xhtml"]);
  });

  it("sumário em nav.xhtml e em toc.ncx, um item por capítulo", async () => {
    const { read, xml } = await open(full);
    const nav = xml(await read("OEBPS/nav.xhtml"));
    const links = [...nav.getElementsByTagName("a")].map((a) => [a.textContent, a.getAttribute("href")]);
    expect(links).toEqual([["Linha 340", "chapter-1.xhtml"], ["A Porta do Elevador", "chapter-2.xhtml"]]);
    const ncx = xml(await read("OEBPS/toc.ncx"));
    expect(ncx.getElementsByTagName("navPoint")).toHaveLength(2);
  });

  it("todo XHTML é bem formado, mesmo com &, < e aspas", async () => {
    const { zip, xml } = await open(full);
    const files = Object.keys(zip.files).filter((f) => f.endsWith(".xhtml") || f.endsWith(".opf") || f.endsWith(".ncx"));
    expect(files.length).toBeGreaterThan(6);
    for (const f of files) {
      const d = xml(await zip.file(f)!.async("string"));
      expect(d.getElementsByTagName("parsererror"), f).toHaveLength(0);
    }
  });

  it("imagem do texto vira figure com img e figcaption; itálico vira em; cenas separadas", async () => {
    const { read, xml } = await open(full);
    const ch = xml(await read("OEBPS/chapter-1.xhtml"));
    const fig = ch.getElementsByTagName("figure")[0];
    expect(fig.getElementsByTagName("img")[0].getAttribute("src")).toMatch(/^images\/img-a\.jpg$/);
    expect(fig.getElementsByTagName("figcaption")[0].textContent).toBe("Mapa & serra");
    expect(ch.getElementsByTagName("em")[0].textContent).toBe("Ele ri");
    expect(ch.documentElement.textContent).toContain("* * *");
    expect(ch.getElementsByTagName("h1")[0].textContent).toBe("Linha 340");
  });

  it("contracapa com sinopse (quebras de linha viram parágrafos) e dedicatória", async () => {
    const { read, xml } = await open(full);
    const back = xml(await read("OEBPS/back.xhtml"));
    expect([...back.getElementsByTagName("p")].map((x) => x.textContent)).toEqual(["João fica milionário em segredo.", "E agora?"]);
    expect(xml(await read("OEBPS/dedication.xhtml")).documentElement.textContent).toContain("Para a Marina");
  });

  it("livro sem autor, sinopse, dedicatória e contracapa ainda é válido", async () => {
    const { read, xml, zip } = await open({ ...full, author: undefined, synopsis: undefined, dedication: undefined, back: undefined, images: new Map() });
    const opf = xml(await read("OEBPS/content.opf"));
    expect(opf.getElementsByTagName("dc:creator")).toHaveLength(0);
    expect(zip.file("OEBPS/back.xhtml")).toBeNull();
    expect(zip.file("OEBPS/dedication.xhtml")).toBeNull();
    // imagem do texto sem bytes: some a imagem, fica a legenda
    const ch = xml(await read("OEBPS/chapter-1.xhtml"));
    expect(ch.getElementsByTagName("img")).toHaveLength(0);
    expect(ch.documentElement.textContent).toContain("Mapa & serra");
  });
});
