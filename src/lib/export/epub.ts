import type { JSONContent } from "@tiptap/core";
import JSZip from "jszip";
import type { ExportBook, ExportOptions } from "./types";

type Img = { mime: string; bytes: Uint8Array };

export type EpubInput = ExportBook & {
  id: string;
  author?: string;
  synopsis?: string;
  dedication?: string;
  cover: Img;
  back?: Img;
  /** Imagens do texto, por id (o que vem depois de `lume-img:`). */
  images: Map<string, Img>;
};

const ext = (mime: string) => (mime === "image/png" ? "png" : mime === "image/gif" ? "gif" : "jpg");

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;");

/** Página XHTML do EPUB 3. */
function page(title: string, body: string, bodyClass = ""): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="pt-BR" lang="pt-BR">
<head>
  <meta charset="UTF-8" />
  <title>${esc(title)}</title>
  <link rel="stylesheet" type="text/css" href="style.css" />
</head>
<body${bodyClass ? ` class="${bodyClass}"` : ""}>
${body}
</body>
</html>
`;
}

function inline(nodes: JSONContent[] = []): string {
  return nodes
    .map((n) => {
      if (n.type === "hardBreak") return "<br />";
      let t = esc(n.text ?? "");
      const marks = new Set((n.marks ?? []).map((m) => m.type));
      if (marks.has("italic")) t = `<em>${t}</em>`;
      if (marks.has("bold")) t = `<strong>${t}</strong>`;
      return t;
    })
    .join("");
}

/** Corpo de uma cena: parágrafos (o primeiro sem recuo) e imagens com legenda. */
function sceneBody(doc: JSONContent, imageHref: (id: string) => string | null): string {
  const out: string[] = [];
  let first = true;
  const walk = (n: JSONContent) => {
    if (n.type === "paragraph") {
      out.push(`<p${first ? ' class="first"' : ""}>${inline(n.content)}</p>`);
      first = false;
    } else if (n.type === "figure") {
      const src = String(n.attrs?.src ?? "");
      const id = src.startsWith("lume-img:") ? src.slice(9) : null;
      const cap = String(n.attrs?.caption ?? "").trim();
      const href = id ? imageHref(id) : null;
      if (href) {
        out.push(`<figure><img src="${href}" alt="${esc(cap || "Imagem")}" />${cap ? `<figcaption>${esc(cap)}</figcaption>` : ""}</figure>`);
      } else if (cap) {
        out.push(`<p class="caption-only">${esc(cap)}</p>`);
      }
      first = true; // depois de uma imagem, o parágrafo recomeça sem recuo
    } else {
      (n.content ?? []).forEach(walk);
    }
  };
  (doc.content ?? []).forEach(walk);
  return out.join("\n");
}

const CSS = `body { font-family: serif; line-height: 1.5; margin: 0 4%; }
p { margin: 0; text-indent: 1.5em; text-align: justify; }
p.first, p.caption-only { text-indent: 0; }
h1 { font-weight: normal; text-align: center; margin: 3em 0 0.4em; font-size: 1.6em; }
h2 { font-weight: normal; font-style: italic; text-align: center; font-size: 1.1em; margin: 2em 0 1em; }
.rule { width: 3em; margin: 0 auto 2.5em; border: 0; border-top: 2px solid #b42318; }
.sep { text-align: center; text-indent: 0; margin: 1.4em 0; letter-spacing: 0.3em; }
figure { margin: 1.4em 0; text-align: center; page-break-inside: avoid; }
figure img { max-width: 100%; height: auto; }
figcaption { font-style: italic; font-size: 0.9em; margin-top: 0.4em; }
.caption-only { text-align: center; font-style: italic; margin: 1em 0; }
body.cover { margin: 0; padding: 0; text-align: center; }
body.cover img { max-width: 100%; max-height: 100%; }
body.title { text-align: center; }
body.title .book { font-size: 2.2em; margin-top: 30%; line-height: 1.2; }
body.title .author { font-size: 1.1em; margin-top: 1.5em; letter-spacing: 0.08em; }
body.dedication p { text-align: right; font-style: italic; margin-top: 35%; text-indent: 0; }
body.back { text-align: center; }
body.back p { text-indent: 0; text-align: justify; margin: 0 0 0.8em; }
body.back img { max-width: 100%; margin-top: 1.5em; }
nav ol { list-style: none; padding: 0; }
nav li { margin: 0.5em 0; }
`;

/** Monta o EPUB 3 (com sumário NCX para leitores antigos, como alguns Kindle). */
export async function toEpub(b: EpubInput, opts: ExportOptions = {}): Promise<Uint8Array> {
  const zip = new JSZip();
  // o mimetype precisa ser a primeira entrada e sem compressão
  zip.file("mimetype", "application/epub+zip", { compression: "STORE" });
  zip.file(
    "META-INF/container.xml",
    `<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml" /></rootfiles>
</container>
`,
  );

  const o = zip.folder("OEBPS")!;
  const manifest: string[] = [];
  const spine: string[] = [];
  const add = (id: string, href: string, type: string, props = "") =>
    manifest.push(`<item id="${id}" href="${href}" media-type="${type}"${props ? ` properties="${props}"` : ""} />`);
  const xhtml = "application/xhtml+xml";

  o.file("style.css", CSS);
  add("css", "style.css", "text/css");

  // capa
  const coverHref = `images/cover.${ext(b.cover.mime)}`;
  o.file(coverHref, b.cover.bytes);
  add("cover-image", coverHref, b.cover.mime, "cover-image");
  o.file("cover.xhtml", page("Capa", `<div><img src="${coverHref}" alt="${esc(b.title)}" /></div>`, "cover"));
  add("cover", "cover.xhtml", xhtml);
  spine.push(`<itemref idref="cover" />`);

  // rosto
  o.file(
    "title.xhtml",
    page(b.title, `<p class="book">${esc(b.title)}</p>${b.author ? `<p class="author">${esc(b.author)}</p>` : ""}`, "title"),
  );
  add("title", "title.xhtml", xhtml);
  spine.push(`<itemref idref="title" />`);

  if (b.dedication?.trim()) {
    o.file("dedication.xhtml", page("Dedicatória", `<p>${esc(b.dedication.trim()).replace(/\n/g, "<br />")}</p>`, "dedication"));
    add("dedication", "dedication.xhtml", xhtml);
    spine.push(`<itemref idref="dedication" />`);
  }

  // imagens do texto (só as que existem)
  const imageHrefs = new Map<string, string>();
  const imageHref = (id: string) => {
    if (imageHrefs.has(id)) return imageHrefs.get(id)!;
    const img = b.images.get(id);
    if (!img) return null;
    const href = `images/${id}.${ext(img.mime)}`;
    o.file(href, img.bytes);
    add(`img-${id}`, href, img.mime);
    imageHrefs.set(id, href);
    return href;
  };

  // capítulos
  const chapters = b.chapters.filter((c) => c.scenes.length);
  const chapterFiles = chapters.map((c, i) => {
    const file = `chapter-${i + 1}.xhtml`;
    const parts = c.scenes.map((s) => {
      const heading = opts.sceneTitles && s.title.trim() && s.title.trim() !== c.title.trim() ? `<h2>${esc(s.title)}</h2>\n` : "";
      return heading + sceneBody(s.doc, imageHref);
    });
    o.file(file, page(c.title, `<section epub:type="chapter">\n<h1>${esc(c.title)}</h1>\n<hr class="rule" />\n${parts.join('\n<p class="sep">* * *</p>\n')}\n</section>`));
    return { file, title: c.title, id: `chapter-${i + 1}` };
  });

  // sumário (EPUB 3) — entra no fluxo de leitura antes dos capítulos
  const navList = chapterFiles.map((c) => `<li><a href="${c.file}">${esc(c.title)}</a></li>`).join("\n");
  o.file("nav.xhtml", page("Sumário", `<nav epub:type="toc" id="toc"><h1>Sumário</h1>\n<ol>\n${navList}\n</ol></nav>`));
  add("nav", "nav.xhtml", xhtml, "nav");
  spine.push(`<itemref idref="nav" />`);
  for (const c of chapterFiles) {
    add(c.id, c.file, xhtml);
    spine.push(`<itemref idref="${c.id}" />`);
  }

  // contracapa
  if (b.synopsis?.trim() || b.back) {
    let backImg = "";
    if (b.back) {
      const href = `images/back.${ext(b.back.mime)}`;
      o.file(href, b.back.bytes);
      add("back-image", href, b.back.mime);
      backImg = `<img src="${href}" alt="Contracapa" />`;
    }
    const synopsis = (b.synopsis ?? "")
      .split(/\n+/)
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => `<p>${esc(l)}</p>`)
      .join("\n");
    o.file("back.xhtml", page("Contracapa", `${synopsis}\n${backImg}`, "back"));
    add("back", "back.xhtml", xhtml);
    spine.push(`<itemref idref="back" />`);
  }

  // sumário antigo (NCX)
  const ncxPoints = chapterFiles
    .map((c, i) => `<navPoint id="np-${i + 1}" playOrder="${i + 1}"><navLabel><text>${esc(c.title)}</text></navLabel><content src="${c.file}" /></navPoint>`)
    .join("\n");
  o.file(
    "toc.ncx",
    `<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head><meta name="dtb:uid" content="urn:uuid:${b.id}" /><meta name="dtb:depth" content="1" /></head>
  <docTitle><text>${esc(b.title)}</text></docTitle>
  <navMap>
${ncxPoints}
  </navMap>
</ncx>
`,
  );
  add("ncx", "toc.ncx", "application/x-dtbncx+xml");

  const modified = new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
  o.file(
    "content.opf",
    `<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="bookid" xml:lang="pt-BR">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="bookid">urn:uuid:${b.id}</dc:identifier>
    <dc:title>${esc(b.title)}</dc:title>
    ${b.author ? `<dc:creator>${esc(b.author)}</dc:creator>` : ""}
    <dc:language>pt-BR</dc:language>
    ${b.synopsis ? `<dc:description>${esc(b.synopsis)}</dc:description>` : ""}
    <meta property="dcterms:modified">${modified}</meta>
    <meta name="cover" content="cover-image" />
  </metadata>
  <manifest>
    ${manifest.join("\n    ")}
  </manifest>
  <spine toc="ncx">
    ${spine.join("\n    ")}
  </spine>
  <guide>
    <reference type="cover" title="Capa" href="cover.xhtml" />
    <reference type="toc" title="Sumário" href="nav.xhtml" />
    ${chapterFiles[0] ? `<reference type="text" title="Início" href="${chapterFiles[0].file}" />` : ""}
  </guide>
</package>
`,
  );

  return zip.generateAsync({ type: "uint8array", mimeType: "application/epub+zip", compression: "DEFLATE" });
}
