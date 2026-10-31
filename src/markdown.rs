//! A Markdown document as two views: the prose units the Simple English profile reads, and the structural facts the
//! cross-document rules read.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use regex::Regex;

use crate::simple::Prose;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    Paragraph,
    Heading,
    Cell,
}

#[derive(Debug)]
pub struct Unit {
    pub kind: UnitKind,
    pub prose: Prose,
}

#[derive(Debug, Clone)]
pub struct Heading {
    pub level: usize,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub line: usize,
    pub cells: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Table {
    /// The heading the table sits under.
    pub section: String,
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
}

impl Table {
    pub fn cell<'a>(&self, row: &'a Row, column: &str) -> Option<&'a str> {
        let i = self
            .columns
            .iter()
            .position(|c| c.eq_ignore_ascii_case(column))?;
        row.cells.get(i).map(String::as_str)
    }
}

#[derive(Debug, Clone)]
pub struct Link {
    pub line: usize,
    pub target: String,
    /// The target resolved against the document's directory. Empty for an external link or an anchor.
    pub resolved: String,
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub line: usize,
    pub kind: String,
    pub values: Vec<String>,
}

/// An identifier such as `ADR-0001`. `before` is the word in front of it, so `games ADR-0001` names another set.
#[derive(Debug, Clone)]
pub struct Mention {
    pub line: usize,
    pub id: String,
    pub before: String,
}

#[derive(Debug, Default)]
pub struct Doc {
    pub header: BTreeMap<String, (usize, String)>,
    pub headings: Vec<Heading>,
    pub tables: Vec<Table>,
    pub links: Vec<Link>,
    pub annotations: Vec<Annotation>,
    pub mentions: Vec<Mention>,
    pub units: Vec<Unit>,
    /// Lines that carry `tablint:allow`. No finding is reported on them.
    pub allowed: BTreeSet<usize>,
}

static HEADING: LazyLock<Regex> = LazyLock::new(|| re(r"^(#{1,6})\s+(.*?)\s*#*\s*$"));
static HEADER_FIELD: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(?:- )?\*\*([A-Za-z-]+):\*\*\s*(.*)$"));
static DELIMITER: LazyLock<Regex> = LazyLock::new(|| re(r"^\|?[\s:|-]+\|?$"));
static LINK: LazyLock<Regex> = LazyLock::new(|| re(r"(^|[^!])\[([^\]]*)\]\(([^)\s]+)\)"));
static CODE_SPAN: LazyLock<Regex> = LazyLock::new(|| re("`[^`]*`"));
static SCHEME: LazyLock<Regex> = LazyLock::new(|| re(r"^[a-z][a-z0-9+.-]*:"));
static ANNOTATION: LazyLock<Regex> =
    LazyLock::new(|| re(r"\(((?:CI|check|enforced|ref): [^)]*)\)"));
static MENTION: LazyLock<Regex> = LazyLock::new(|| re(r"\b([A-Z]{2,}-\d{3,4})\b"));
static WORD_BEFORE: LazyLock<Regex> = LazyLock::new(|| re(r"([A-Za-z]+)\W*$"));
static FENCE: LazyLock<Regex> = LazyLock::new(|| re(r"^\s*(```|~~~)"));

fn re(p: &str) -> Regex {
    Regex::new(p).expect("a built-in pattern compiles")
}

pub fn parse(path: &str, text: &str) -> Doc {
    let mut doc = Doc::default();
    scan_lines(path, text, &mut doc);
    doc.units = units(text);
    doc
}

/// The structural facts, read line by line. A fenced code block holds no facts.
fn scan_lines(path: &str, text: &str, doc: &mut Doc) {
    let mut section = String::new();
    let mut table: Option<usize> = None;
    let mut in_fence = false;
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if FENCE.is_match(line) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if line.contains("tablint:allow") {
            doc.allowed.insert(n);
        }
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            table = None;
        }
        if let Some(h) = HEADING.captures(line) {
            h[2].clone_into(&mut section);
            doc.headings.push(Heading {
                level: h[1].len(),
                text: section.clone(),
            });
        } else if let Some(f) = HEADER_FIELD
            .captures(trimmed)
            .filter(|_| doc.headings.iter().all(|h| h.level == 1))
        {
            doc.header
                .entry(f[1].to_lowercase())
                .or_insert((n, f[2].trim().to_owned()));
        } else if trimmed.starts_with('|') {
            match table {
                None => {
                    doc.tables.push(Table {
                        section: section.clone(),
                        columns: cells(trimmed),
                        rows: Vec::new(),
                    });
                    table = Some(doc.tables.len() - 1);
                }
                Some(t) if !DELIMITER.is_match(trimmed) => doc.tables[t].rows.push(Row {
                    line: n,
                    cells: cells(trimmed),
                }),
                Some(_) => {}
            }
        }
        doc.annotations.extend(annotations(n, line));
        doc.mentions.extend(mentions(n, line));
        doc.links.extend(links(path, n, line));
    }
}

fn cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|c| c.trim().to_owned())
        .collect()
}

/// Splits `(CI: a, b; ref: c)` into one fact per kind.
fn annotations(n: usize, line: &str) -> Vec<Annotation> {
    let mut out = Vec::new();
    for m in ANNOTATION.captures_iter(line) {
        for part in m[1].split(';') {
            let (kind, values) = part.trim().split_once(": ").unwrap_or((part.trim(), ""));
            let values = values
                .split(',')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .collect();
            out.push(Annotation {
                line: n,
                kind: kind.to_owned(),
                values,
            });
        }
    }
    out
}

fn mentions(n: usize, line: &str) -> Vec<Mention> {
    MENTION
        .captures_iter(line)
        .map(|c| {
            let m = c.get(1).expect("the pattern has a group");
            let before = WORD_BEFORE
                .captures(&line[..m.start()])
                .map(|w| w[1].to_lowercase())
                .unwrap_or_default();
            Mention {
                line: n,
                id: m.as_str().to_owned(),
                before,
            }
        })
        .collect()
}

/// A link inside a code span is code, not a link.
fn links(path: &str, n: usize, line: &str) -> Vec<Link> {
    let line = CODE_SPAN.replace_all(line, "");
    LINK.captures_iter(&line)
        .map(|c| {
            let target = c[3].to_owned();
            let resolved = if SCHEME.is_match(&target) || target.starts_with('#') {
                String::new()
            } else {
                resolve(path, target.split('#').next().unwrap_or_default())
            };
            Link {
                line: n,
                target,
                resolved,
            }
        })
        .collect()
}

/// Joins a relative target to the document's directory and removes `.` and `..`. A target above the root keeps
/// its leading `..`, so a rule can tell it apart.
pub fn resolve(from: &str, target: &str) -> String {
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop();
    for p in target.split('/') {
        match p {
            "" | "." => {}
            ".." if parts.last().is_some_and(|l| *l != "..") => {
                parts.pop();
            }
            _ => parts.push(p),
        }
    }
    parts.join("/")
}

/// The prose units: each paragraph, heading, list item, and table cell is its own text, per ADR-0001.
fn units(text: &str) -> Vec<Unit> {
    let starts: Vec<usize> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let line_of = |offset: usize| starts.partition_point(|&s| s <= offset);
    let mut b = Builder::default();
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES;
    for (event, range) in Parser::new_ext(text, opts).into_offset_iter() {
        let line = line_of(range.start);
        match event {
            Event::Start(Tag::Paragraph | Tag::Item) => b.open(UnitKind::Paragraph),
            Event::Start(Tag::Heading { .. }) => b.open(UnitKind::Heading),
            Event::Start(Tag::TableCell) => b.open(UnitKind::Cell),
            Event::End(
                TagEnd::Paragraph | TagEnd::Item | TagEnd::Heading(_) | TagEnd::TableCell,
            ) => b.close(),
            Event::Start(Tag::CodeBlock(_) | Tag::HtmlBlock | Tag::MetadataBlock(_)) => b.skip += 1,
            Event::End(TagEnd::CodeBlock | TagEnd::HtmlBlock | TagEnd::MetadataBlock(_)) => {
                b.skip -= 1;
            }
            Event::Start(Tag::Link { .. }) => b
                .links
                .push(b.current.as_ref().map_or(0, |u| u.prose.text.len())),
            Event::End(TagEnd::Link) => b.end_link(line),
            Event::Text(t) if b.skip == 0 => b.push(&t, line),
            // A code span is one word, and its punctuation is code, per ADR-0001.
            Event::Code(_) | Event::InlineMath(_) | Event::DisplayMath(_) => b.push("x", line),
            Event::SoftBreak | Event::HardBreak => b.push(" ", line),
            _ => {}
        }
    }
    b.close();
    b.units
}

#[derive(Default)]
struct Builder {
    units: Vec<Unit>,
    current: Option<Unit>,
    /// The text position where each open link starts.
    links: Vec<usize>,
    skip: usize,
}

impl Builder {
    fn open(&mut self, kind: UnitKind) {
        self.close();
        self.current = Some(Unit {
            kind,
            prose: Prose::default(),
        });
    }

    fn close(&mut self) {
        if let Some(u) = self.current.take().filter(|u| !u.prose.is_blank()) {
            self.units.push(u);
        }
    }

    fn push(&mut self, s: &str, line: usize) {
        if let Some(u) = self.current.as_mut() {
            u.prose.push(s, line);
        }
    }

    /// A link whose text is a path or a file name is code, and counts as one word.
    fn end_link(&mut self, line: usize) {
        let Some(start) = self.links.pop() else {
            return;
        };
        let Some(u) = self.current.as_mut() else {
            return;
        };
        let Some(label) = u.prose.text.get(start..) else {
            return;
        };
        if !label.contains(' ') && label.contains(['/', '.']) {
            u.prose.truncate(start);
            u.prose.push("x", line);
        }
    }
}

#[cfg(test)]
#[allow(clippy::assert_is_empty)]
mod tests {
    use super::*;

    #[test]
    fn units_strip_markup() {
        let doc = parse(
            "docs/a.md",
            "# Title\n\nA **bold** `code; here` and [docs/x.md](x.md) link.\n\n| A | B |\n| --- | --- |\n| one | two |\n\n1. Step one.\n",
        );
        let texts: Vec<_> = doc
            .units
            .iter()
            .map(|u| (u.kind, u.prose.text.trim().to_owned()))
            .collect();
        assert_eq!(texts[0], (UnitKind::Heading, "Title".into()));
        assert_eq!(
            texts[1],
            (UnitKind::Paragraph, "A bold x and x link.".into())
        );
        assert!(texts.contains(&(UnitKind::Cell, "two".into())));
        assert!(texts.contains(&(UnitKind::Paragraph, "Step one.".into())));
        assert_eq!(doc.units[1].prose.first_line(), 3);
    }

    #[test]
    fn facts() {
        let text = "# ADR-0001: X\n\n- **Status:** Accepted\n**Decides:** One thing.\n\n## Context\n\nSee games ADR-0002 and ADR-0003, per [b](../b.md#a). `(CI: lint, test; ref: RFC 1)`\n\n```\n## Not a heading\n```\n";
        let doc = parse("docs/adr/0001-x.md", text);
        assert_eq!(doc.header["status"], (3, "Accepted".into()));
        assert_eq!(doc.header["decides"].1, "One thing.");
        assert_eq!(doc.headings.len(), 2);
        assert_eq!(
            doc.mentions
                .iter()
                .map(|m| (m.id.as_str(), m.before.as_str()))
                .collect::<Vec<_>>()[1..],
            [("ADR-0002", "games"), ("ADR-0003", "and")]
        );
        assert_eq!(doc.links[0].resolved, "docs/b.md");
        assert_eq!(doc.annotations.len(), 2);
        assert_eq!(doc.annotations[0].values, ["lint", "test"]);
    }

    #[test]
    fn resolve_paths() {
        assert_eq!(resolve("docs/adr/a.md", "../../README.md"), "README.md");
        assert_eq!(resolve("README.md", "../x.md"), "../x.md");
    }
}
