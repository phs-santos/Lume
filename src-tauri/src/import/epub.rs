//! Lê um EPUB e transforma em capítulos e cenas do Lume.

use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub text: String,
    pub italic: bool,
    pub bold: bool,
}

/// Um parágrafo é uma sequência de trechos; `None` marca uma quebra de linha dentro dele.
pub type Paragraph = Vec<Option<Run>>;

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedScene {
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedChapter {
    pub title: String,
    pub scenes: Vec<ImportedScene>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedBook {
    pub title: String,
    pub chapters: Vec<ImportedChapter>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum EpubError {
    #[error("NOT_EPUB: {0}")]
    NotEpub(String),
    #[error("EMPTY_EPUB")]
    Empty,
}

use std::collections::HashMap;
use std::io::Read;

fn not_epub(e: impl std::fmt::Display) -> EpubError {
    EpubError::NotEpub(e.to_string())
}

fn read_entry(z: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>, name: &str) -> Result<String, EpubError> {
    let mut f = z.by_name(name).map_err(|e| not_epub(format!("{name}: {e}")))?;
    let mut s = String::new();
    f.read_to_string(&mut s).map_err(not_epub)?;
    Ok(s)
}

/// Entidades HTML comuns que não existem em XML puro.
fn fix_entities(s: &str) -> String {
    const MAP: &[(&str, &str)] = &[
        ("&nbsp;", "&#160;"), ("&mdash;", "&#8212;"), ("&ndash;", "&#8211;"), ("&hellip;", "&#8230;"),
        ("&laquo;", "&#171;"), ("&raquo;", "&#187;"), ("&ldquo;", "&#8220;"), ("&rdquo;", "&#8221;"),
        ("&lsquo;", "&#8216;"), ("&rsquo;", "&#8217;"), ("&copy;", "&#169;"), ("&shy;", "&#173;"),
    ];
    MAP.iter().fold(s.to_string(), |acc, (from, to)| acc.replace(from, to))
}

fn parse_xml(text: &str) -> Result<roxmltree::Document<'_>, EpubError> {
    let opts = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    roxmltree::Document::parse_with_options(text, opts).map_err(not_epub)
}

fn epub_type(n: &roxmltree::Node) -> String {
    n.attributes().filter(|a| a.name() == "type").map(|a| a.value().to_string()).collect::<Vec<_>>().join(" ")
}

/// Junta espaços como o HTML faz, mas preserva o espaço inseparável.
fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut space = false;
    for ch in s.chars() {
        if matches!(ch, ' ' | '\t' | '\n' | '\r') {
            if !space {
                out.push(' ');
            }
            space = true;
        } else {
            out.push(ch);
            space = false;
        }
    }
    out
}

fn collect_runs(n: roxmltree::Node, italic: bool, bold: bool, out: &mut Paragraph) {
    for c in n.children() {
        if c.is_text() {
            out.push(Some(Run { text: collapse(c.text().unwrap_or("")), italic, bold }));
        } else if c.is_element() {
            match c.tag_name().name() {
                "br" => out.push(None),
                "em" | "i" | "cite" => collect_runs(c, true, bold, out),
                "strong" | "b" => collect_runs(c, italic, true, out),
                _ => collect_runs(c, italic, bold, out),
            }
        }
    }
}

/// Limpa as bordas, junta trechos iguais vizinhos e descarta vazios.
fn tidy(mut p: Paragraph) -> Paragraph {
    let mut merged: Paragraph = Vec::new();
    for r in p.drain(..) {
        match (merged.last_mut(), r) {
            (Some(Some(last)), Some(r)) if last.italic == r.italic && last.bold == r.bold => last.text.push_str(&r.text),
            (_, r) => merged.push(r),
        }
    }
    // espaço logo antes/depois de uma quebra e nas pontas não conta
    for i in 0..merged.len() {
        let before_break = merged.get(i + 1).map_or(true, |x| x.is_none());
        let after_break = i == 0 || merged[i - 1].is_none();
        if let Some(r) = merged[i].as_mut() {
            if after_break {
                r.text = r.text.trim_start_matches(' ').to_string();
            }
            if before_break {
                r.text = r.text.trim_end_matches(' ').to_string();
            }
        }
    }
    merged.retain(|r| r.as_ref().map_or(true, |r| !r.text.is_empty()));
    merged
}

fn is_separator(text: &str, class: &str) -> bool {
    let t: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    class.contains("separador") || class.contains("scene-break") || matches!(t.as_str(), "***" | "⁂" | "#" | "◆" | "~")
}

struct Walk {
    title: Option<String>,
    scenes: Vec<ImportedScene>,
}

impl Walk {
    fn cut(&mut self) {
        if self.scenes.last().map_or(true, |s| !s.paragraphs.is_empty()) {
            self.scenes.push(ImportedScene { paragraphs: vec![] });
        }
    }

    fn visit(&mut self, n: roxmltree::Node, in_header: bool) {
        for c in n.children().filter(|c| c.is_element()) {
            match c.tag_name().name() {
                "header" => self.visit(c, true),
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    if self.title.is_none() {
                        let t = collapse(&c.descendants().filter(|d| d.is_text()).filter_map(|d| d.text()).collect::<String>()).trim().to_string();
                        if !t.is_empty() {
                            self.title = Some(t);
                        }
                    }
                }
                "hr" => self.cut(),
                "p" if !in_header => {
                    let text: String = c.descendants().filter(|d| d.is_text()).filter_map(|d| d.text()).collect();
                    if is_separator(&text, c.attribute("class").unwrap_or("")) {
                        self.cut();
                        continue;
                    }
                    let mut runs = Vec::new();
                    collect_runs(c, false, false, &mut runs);
                    let p = tidy(runs);
                    if p.iter().any(|r| r.is_some()) {
                        self.scenes.last_mut().unwrap().paragraphs.push(p);
                    }
                }
                "p" | "nav" | "script" | "style" | "aside" => {}
                _ => self.visit(c, in_header),
            }
        }
    }
}

pub fn parse_epub(bytes: &[u8]) -> Result<ImportedBook, EpubError> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(not_epub)?;
    let container = read_entry(&mut z, "META-INF/container.xml")?;
    let opf_path = parse_xml(&container)?
        .descendants()
        .find(|n| n.has_tag_name("rootfile"))
        .and_then(|n| n.attribute("full-path"))
        .map(String::from)
        .ok_or_else(|| not_epub("sem rootfile"))?;
    let base = opf_path.rsplit_once('/').map(|(d, _)| format!("{d}/")).unwrap_or_default();
    let opf_text = read_entry(&mut z, &opf_path)?;
    let opf = parse_xml(&opf_text)?;
    let book_title = opf
        .descendants()
        .find(|n| n.tag_name().name() == "title")
        .and_then(|n| n.text())
        .map(|t| collapse(t).trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "Livro importado".into());
    let manifest: HashMap<&str, (&str, &str)> = opf
        .descendants()
        .filter(|n| n.has_tag_name("item"))
        .filter_map(|n| Some((n.attribute("id")?, (n.attribute("href")?, n.attribute("properties").unwrap_or("")))))
        .collect();
    let spine: Vec<String> = opf
        .descendants()
        .filter(|n| n.has_tag_name("itemref"))
        .filter_map(|n| n.attribute("idref"))
        .filter_map(|id| manifest.get(id))
        .filter(|(_, props)| !props.contains("nav"))
        .map(|(href, _)| format!("{base}{href}"))
        .collect();

    let mut chapters = Vec::new();
    for path in spine {
        let raw = read_entry(&mut z, &path)?;
        let text = fix_entities(&raw);
        let doc = parse_xml(&text)?;
        let Some(body) = doc.descendants().find(|n| n.has_tag_name("body") || n.tag_name().name() == "body") else { continue };
        let front = epub_type(&body).contains("frontmatter")
            || body.descendants().any(|d| ["titlepage", "toc", "copyright-page"].iter().any(|t| epub_type(&d).contains(t)) || d.tag_name().name() == "nav");
        if front {
            continue;
        }
        let mut w = Walk { title: None, scenes: vec![ImportedScene { paragraphs: vec![] }] };
        w.visit(body, false);
        let scenes: Vec<_> = w.scenes.into_iter().filter(|s| !s.paragraphs.is_empty()).collect();
        if scenes.is_empty() {
            continue;
        }
        let fallback = doc.descendants().find(|n| n.tag_name().name() == "title").and_then(|n| n.text()).map(|t| t.trim().to_string());
        let title = w.title.or(fallback).filter(|t| !t.is_empty()).unwrap_or_else(|| format!("Capítulo {}", chapters.len() + 1));
        chapters.push(ImportedChapter { title, scenes });
    }
    if chapters.is_empty() {
        return Err(EpubError::Empty);
    }
    Ok(ImportedBook { title: book_title, chapters })
}

/// Documento TipTap de uma cena.
pub fn scene_doc(s: &ImportedScene) -> Value {
    let paras: Vec<Value> = s
        .paragraphs
        .iter()
        .map(|p| {
            let content: Vec<Value> = p
                .iter()
                .map(|r| match r {
                    None => json!({"type": "hardBreak"}),
                    Some(r) => {
                        let mut marks = vec![];
                        if r.italic {
                            marks.push(json!({"type": "italic"}));
                        }
                        if r.bold {
                            marks.push(json!({"type": "bold"}));
                        }
                        if marks.is_empty() { json!({"type": "text", "text": r.text}) } else { json!({"type": "text", "text": r.text, "marks": marks}) }
                    }
                })
                .collect();
            json!({"type": "paragraph", "content": content})
        })
        .collect();
    json!({"type": "doc", "content": paras})
}

/// Texto puro da cena (para contar palavras).
pub fn scene_text(s: &ImportedScene) -> String {
    s.paragraphs
        .iter()
        .map(|p| p.iter().map(|r| r.as_ref().map_or("\n", |r| r.text.as_str())).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Write;

    /// Monta um EPUB pequeno em memória.
    pub(crate) fn sample_epub() -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::SimpleFileOptions::default();
            let mut add = |name: &str, body: &str| {
                z.start_file(name, o).unwrap();
                z.write_all(body.as_bytes()).unwrap();
            };
            add("mimetype", "application/epub+zip");
            add("META-INF/container.xml", r#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#);
            add("OEBPS/content.opf", r#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Até ter certeza</dc:title></metadata>
              <manifest><item id="rosto" href="rosto.xhtml" media-type="application/xhtml+xml"/><item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
              <item id="c1" href="cap/um.xhtml" media-type="application/xhtml+xml"/><item id="c2" href="cap/dois.xhtml" media-type="application/xhtml+xml"/></manifest>
              <spine><itemref idref="rosto"/><itemref idref="nav"/><itemref idref="c1"/><itemref idref="c2"/></spine></package>"#);
            add("OEBPS/rosto.xhtml", r#"<?xml version="1.0"?><!DOCTYPE html><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><body epub:type="frontmatter"><section epub:type="titlepage"><h1>Até ter certeza</h1><p>Hefesto</p></section></body></html>"#);
            add("OEBPS/nav.xhtml", r#"<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><body><nav><ol><li>Linha 340</li></ol></nav></body></html>"#);
            add("OEBPS/cap/um.xhtml", r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE html><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><head><title>Linha 340</title></head>
              <body epub:type="bodymatter"><section epub:type="chapter"><header class="abertura"><p class="numero">Capítulo 1</p><h1>Linha 340</h1></header>
              <p class="primeiro">A tela é a única luz do quarto.</p>
              <p>&quot;kkkk&quot; manda o <em>Carlos</em>, e eu <strong>rio</strong>.</p>
              <p>Primeira linha<br/>segunda linha</p></section></body></html>"#);
            add("OEBPS/cap/dois.xhtml", r#"<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><body><h2>A carta</h2>
              <p>Antes&nbsp;do corte.</p><hr/><p>Depois do corte.</p><p class="separador">* * *</p><p>Terceira cena.</p><p>   </p></body></html>"#);
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    fn plain(p: &Paragraph) -> String {
        p.iter().map(|r| r.as_ref().map(|r| r.text.as_str()).unwrap_or("\n")).collect()
    }

    #[test]
    fn reads_title_and_skips_title_page_and_nav() {
        let b = parse_epub(&sample_epub()).unwrap();
        assert_eq!(b.title, "Até ter certeza");
        assert_eq!(b.chapters.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(), ["Linha 340", "A carta"]);
    }

    #[test]
    fn header_is_not_text_and_marks_are_kept() {
        let b = parse_epub(&sample_epub()).unwrap();
        let ps = &b.chapters[0].scenes[0].paragraphs;
        assert_eq!(ps.len(), 3);
        assert_eq!(plain(&ps[0]), "A tela é a única luz do quarto.");
        assert_eq!(plain(&ps[1]), "\"kkkk\" manda o Carlos, e eu rio.");
        assert!(ps[1].iter().flatten().any(|r| r.text == "Carlos" && r.italic && !r.bold));
        assert!(ps[1].iter().flatten().any(|r| r.text == "rio" && r.bold));
        assert_eq!(plain(&ps[2]), "Primeira linha\nsegunda linha");
    }

    #[test]
    fn hr_and_asterisks_split_scenes_and_entities_work() {
        let b = parse_epub(&sample_epub()).unwrap();
        let scenes = &b.chapters[1].scenes;
        assert_eq!(scenes.len(), 3);
        assert_eq!(plain(&scenes[0].paragraphs[0]), "Antes\u{a0}do corte.");
        assert_eq!(plain(&scenes[2].paragraphs[0]), "Terceira cena.");
        assert_eq!(scenes[2].paragraphs.len(), 1); // parágrafo vazio descartado
    }

    #[test]
    fn scene_doc_is_tiptap_with_marks_and_breaks() {
        let b = parse_epub(&sample_epub()).unwrap();
        let doc = scene_doc(&b.chapters[0].scenes[0]);
        assert_eq!(doc["type"], "doc");
        let p1 = &doc["content"][1]["content"];
        assert!(p1.as_array().unwrap().iter().any(|n| n["text"] == "Carlos" && n["marks"][0]["type"] == "italic"));
        let p2 = &doc["content"][2]["content"];
        assert_eq!(p2[1]["type"], "hardBreak");
        assert_eq!(scene_text(&b.chapters[0].scenes[0]).split_whitespace().count(), 19);
    }

    #[test]
    fn garbage_is_not_epub() {
        assert!(matches!(parse_epub(b"isto nao e um zip"), Err(EpubError::NotEpub(_))));
    }
}
