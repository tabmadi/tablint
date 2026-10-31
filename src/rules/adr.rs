//! The ADR set: its header, its index, its references, and its section order, per ADR-0001.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use regex::Regex;

use super::Ctx;
use crate::finding::Finding;
use crate::markdown::Doc;

static INDEX_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[(\d{4})\]\(").expect("a built-in pattern compiles"));
static NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d{4}$").expect("a built-in pattern compiles"));
static SENTENCE_BREAK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.\s+\S").expect("a built-in pattern compiles"));

struct Set<'a> {
    /// ADR number to path.
    adrs: BTreeMap<String, &'a str>,
    /// Index rows: line, number, and the Decides cell.
    rows: Vec<(usize, String, Option<String>)>,
    reserved: BTreeSet<String>,
}

fn load<'a>(ctx: &'a Ctx<'_>) -> Set<'a> {
    let dir = ctx.cfg.adr.dir.trim_end_matches('/');
    let file = Regex::new(&format!(
        r"^{}/(\d{{4}})-[a-z0-9-]+\.md$",
        regex::escape(dir)
    ))
    .expect("an escaped path compiles");
    let adrs = ctx
        .docs
        .keys()
        .filter_map(|p| file.captures(p).map(|c| (c[1].to_owned(), *p)))
        .collect();
    let mut rows = Vec::new();
    let mut reserved = BTreeSet::new();
    if let Some(index) = ctx.docs.get(ctx.cfg.adr.index.as_str()) {
        for t in &index.tables {
            for r in &t.rows {
                let first = r.cells.first().map_or("", String::as_str);
                if let Some(c) = INDEX_ID.captures(first) {
                    rows.push((
                        r.line,
                        c[1].to_owned(),
                        t.cell(r, "Decides").map(str::to_owned),
                    ));
                } else if t.section.starts_with("Reserved") && NUMBER.is_match(first) {
                    reserved.insert(first.to_owned());
                }
            }
        }
    }
    Set {
        adrs,
        rows,
        reserved,
    }
}

pub fn check(ctx: &Ctx<'_>) -> Vec<Finding> {
    let cfg = &ctx.cfg.adr;
    let set = load(ctx);
    if set.adrs.is_empty() {
        return Vec::new();
    }
    let index = cfg.index.as_str();
    let mut out = Vec::new();
    let indexed: BTreeSet<&str> = set.rows.iter().map(|(_, id, _)| id.as_str()).collect();
    for (id, &path) in &set.adrs {
        let doc = &ctx.docs[path];
        out.extend(header(path, doc, &cfg.header_fields));
        if !indexed.contains(id.as_str()) {
            out.push(Finding::new(
                index,
                0,
                "adr/index",
                format!("ADR-{id} has no row"),
            ));
        }
        if !cfg.sections_exempt.contains(id) {
            out.extend(sections(path, doc, &cfg.sections));
        }
        if let Some((line, _)) = doc.header.get("related") {
            for m in doc.mentions.iter().filter(|m| m.line == *line) {
                if let Some(num) = local(m.id.as_str(), &m.before, &cfg.foreign)
                    && !set.adrs.contains_key(num)
                {
                    out.push(Finding::new(
                        path,
                        *line,
                        "adr/related",
                        format!("Related names ADR-{num}, which has no file"),
                    ));
                }
            }
        }
    }
    for (line, id, decides) in &set.rows {
        let Some(&path) = set.adrs.get(id) else {
            out.push(Finding::new(
                index,
                *line,
                "adr/index",
                format!("the row names ADR-{id}, which has no file"),
            ));
            continue;
        };
        let header = ctx.docs[path]
            .header
            .get("decides")
            .map(|(_, v)| v.as_str());
        if let (true, Some(row), Some(header)) = (cfg.index_matches_decides, decides, header)
            && trim(row) != trim(header)
        {
            out.push(Finding::new(
                index,
                *line,
                "adr/index-decides",
                format!("the row does not repeat the Decides line of {path}"),
            ));
        }
    }
    for id in set.reserved.iter().filter(|id| set.adrs.contains_key(*id)) {
        out.push(Finding::new(
            index,
            0,
            "adr/index",
            format!("ADR-{id} is both a file and reserved"),
        ));
    }
    out.extend(references(ctx, &set));
    out
}

fn header(path: &str, doc: &Doc, fields: &[String]) -> Vec<Finding> {
    let mut out: Vec<_> = fields
        .iter()
        .filter(|f| !doc.header.contains_key(f.as_str()))
        .map(|f| {
            Finding::new(
                path,
                0,
                "adr/header",
                format!("the header has no `{f}` field"),
            )
        })
        .collect();
    if let Some((line, decides)) = doc.header.get("decides")
        && SENTENCE_BREAK.is_match(decides)
    {
        out.push(Finding::new(
            path,
            *line,
            "adr/decides",
            "the Decides line has more than one sentence",
        ));
    }
    out
}

fn sections(path: &str, doc: &Doc, order: &[String]) -> Vec<Finding> {
    let present: Vec<&str> = doc
        .headings
        .iter()
        .filter(|h| h.level == 2)
        .map(|h| h.text.as_str())
        .collect();
    let known: Vec<&str> = present
        .iter()
        .copied()
        .filter(|s| order.iter().any(|o| o == s))
        .collect();
    let expected: Vec<&str> = order
        .iter()
        .map(String::as_str)
        .filter(|o| known.contains(o))
        .collect();
    let mut out = Vec::new();
    if known != expected {
        out.push(Finding::new(
            path,
            0,
            "adr/sections",
            format!("the sections are out of order: {}", known.join(", ")),
        ));
    }
    if order.last().is_some_and(|l| l == "Rules") && present.last().is_some_and(|l| *l != "Rules") {
        out.push(Finding::new(
            path,
            0,
            "adr/sections",
            "the last section is not Rules",
        ));
    }
    out
}

/// Every `ADR-NNNN` in a reader document names a file or a reserved number.
fn references(ctx: &Ctx<'_>, set: &Set<'_>) -> Vec<Finding> {
    let Ok(readers) = crate::config::Paths::new(&ctx.cfg.adr.reader_paths) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (&path, doc) in ctx.docs.iter().filter(|(p, _)| readers.matches(p)) {
        for m in &doc.mentions {
            let Some(num) = local(&m.id, &m.before, &ctx.cfg.adr.foreign) else {
                continue;
            };
            if !set.adrs.contains_key(num) && !set.reserved.contains(num) {
                out.push(Finding::new(
                    path,
                    m.line,
                    "adr/reference",
                    format!("ADR-{num} is neither a file nor reserved"),
                ));
            }
        }
    }
    out
}

/// The number of a local ADR mention, or `None` for another identifier or another repository's ADR.
fn local<'m>(id: &'m str, before: &str, foreign: &[String]) -> Option<&'m str> {
    let num = id.strip_prefix("ADR-")?;
    (!foreign.iter().any(|f| f.eq_ignore_ascii_case(before))).then_some(num)
}

fn trim(s: &str) -> &str {
    s.trim().trim_end_matches('.')
}
