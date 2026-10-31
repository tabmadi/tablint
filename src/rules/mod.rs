//! The rules, grouped by what they read. Each rule ID is `<group>/<name>`, and `disable` in `tablint.toml` takes
//! either form.

mod adr;
mod chars;
mod comment;
mod docs;
mod messages;
mod prose;

use std::collections::BTreeMap;

use anyhow::Result;

use crate::config::{Config, Paths};
use crate::finding::Finding;
use crate::markdown::{self, Doc};
use crate::repo::Repo;

/// Every rule, with what it enforces. `tablint rules` prints this table.
pub const RULES: &[(&str, &str)] = &[
    (
        "simple/semicolon",
        "Prose has no semicolon. Write two sentences, or use a colon",
    ),
    (
        "simple/parentheses",
        "Prose has no parentheses. An aside is its own sentence",
    ),
    ("simple/question", "Prose has no question. Write the answer"),
    ("simple/exclamation", "Prose has no exclamation mark"),
    ("simple/ampersand", "Prose writes `and`, not `&`"),
    (
        "simple/quote",
        "Prose has no quote marks. Literal text is in backticks",
    ),
    ("simple/dash", "Prose has no dash between words"),
    ("simple/slash", "Prose writes `or` or `and`, not a slash"),
    ("simple/emoji", "Prose has no emoji"),
    ("simple/contraction", "Prose has no contractions"),
    (
        "simple/latin",
        "Prose has no Latin abbreviations such as `e.g.`",
    ),
    (
        "simple/complex-word",
        "Prose uses the common word, such as `use` for `utilize`",
    ),
    (
        "simple/sentence-length",
        "A sentence has at most 25 words, or 20 in a procedure",
    ),
    (
        "construct/intensifier",
        "No intensifiers such as `very` or `just`",
    ),
    ("construct/hedge", "No hedges such as `basically`"),
    (
        "construct/meta-commentary",
        "No introductions such as `note that`",
    ),
    (
        "construct/status",
        "A document states no implementation status such as `not yet`",
    ),
    (
        "construct/chronology",
        "A text states what is true now, not how it came to be",
    ),
    (
        "construct/planned-work",
        "A document holds no `TODO` or follow-up list",
    ),
    ("construct/dated-heading", "A heading has no year"),
    (
        "char/banned",
        "No em dash, en dash, ellipsis, or curly quote in any text file",
    ),
    ("comment/length", "A comment block has at most three lines"),
    ("comment/paragraphs", "A comment block is one paragraph"),
    ("comment/commented-out-code", "No commented-out code"),
    (
        "comment/changelog",
        "No author, date, or changelog comments",
    ),
    ("comment/banner", "No decorative banners"),
    ("comment/todo", "A `TODO` cites an ADR or an issue"),
    (
        "comment/echo-doc",
        "A doc comment states a fact the signature cannot",
    ),
    ("adr/header", "An ADR header has every configured field"),
    ("adr/decides", "The Decides line is one sentence"),
    (
        "adr/index",
        "Every ADR has an index row, and every index row has an ADR",
    ),
    (
        "adr/index-decides",
        "The index row repeats the Decides line",
    ),
    (
        "adr/reference",
        "Every `ADR-NNNN` names an ADR file or a reserved number",
    ),
    ("adr/related", "Every ADR in the Related header exists"),
    (
        "adr/sections",
        "ADR sections keep the configured order and end with Rules",
    ),
    (
        "docs/genre",
        "A document under docs/ sits in a genre directory",
    ),
    (
        "docs/root",
        "The docs/ root holds only the configured entry documents and registries",
    ),
    (
        "docs/filename",
        "No two documents under docs/ share a filename",
    ),
    (
        "docs/single-file-dir",
        "No directory under docs/ holds a single file",
    ),
    (
        "docs/untracked-link",
        "No link points to a file that git does not track",
    ),
    (
        "docs/forbidden-link",
        "No link goes from the configured documents to a forbidden target",
    ),
    (
        "annotation/task",
        "A `(CI: <task>)` annotation names a mise task that exists",
    ),
];

pub struct Ctx<'a> {
    pub repo: &'a Repo,
    pub cfg: &'a Config,
    /// Every Markdown file a rule may read, parsed once.
    pub docs: BTreeMap<&'a str, Doc>,
    /// The files no rule reads: excluded, adopted verbatim, or a lockfile.
    pub skipped: Paths,
}

impl Ctx<'_> {
    /// The files a rule reads, in order.
    pub fn files(&self) -> impl Iterator<Item = &str> {
        self.repo
            .files
            .iter()
            .map(String::as_str)
            .filter(|f| !self.skipped.matches(f))
    }
}

/// Runs every enabled rule and returns the findings, sorted, with the allowed lines removed.
pub fn run(repo: &Repo, cfg: &Config) -> Result<Vec<Finding>> {
    let mut skipped: Vec<String> = cfg.exclude.clone();
    skipped.extend(cfg.prose.verbatim.iter().cloned());
    skipped.extend(
        [
            "**/*.lock",
            "**/go.sum",
            "**/go.work.sum",
            "**/package-lock.json",
        ]
        .map(String::from),
    );
    let excluded = Paths::new(&cfg.exclude)?;
    let mut ctx = Ctx {
        repo,
        cfg,
        docs: BTreeMap::new(),
        skipped: Paths::new(&skipped)?,
    };
    for f in repo
        .files
        .iter()
        .filter(|f| f.ends_with(".md") && !excluded.matches(f))
    {
        if let Some(text) = repo.read(f) {
            ctx.docs.insert(f.as_str(), markdown::parse(f, &text));
        }
    }
    let mut found = Vec::new();
    found.extend(prose::check(&ctx)?);
    found.extend(chars::check(&ctx));
    found.extend(comment::check(&ctx)?);
    found.extend(adr::check(&ctx));
    found.extend(docs::check(&ctx)?);
    found.extend(messages::check(&ctx)?);
    found.retain(|f| cfg.enabled(f.rule) && !allowed(&ctx, f));
    found.sort();
    found.dedup();
    Ok(found)
}

/// A line that carries `tablint:allow` opts out of every rule.
fn allowed(ctx: &Ctx<'_>, f: &Finding) -> bool {
    if let Some(doc) = ctx.docs.get(f.path.as_str()) {
        return doc.allowed.contains(&f.line);
    }
    f.line > 0
        && ctx.repo.read(&f.path).is_some_and(|t| {
            t.lines()
                .nth(f.line - 1)
                .is_some_and(|l| l.contains("tablint:allow"))
        })
}
