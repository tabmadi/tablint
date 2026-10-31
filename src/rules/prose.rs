//! The Simple English profile and the banned constructs, over every Markdown document.

use std::sync::LazyLock;

use anyhow::{Context, Result};
use regex::Regex;

use super::Ctx;
use crate::config::Paths;
use crate::constructs::{self, Scope};
use crate::finding::Finding;
use crate::markdown::UnitKind;
use crate::simple::{self, Options};

/// An enforcement annotation is a pointer to a mechanism, not prose.
static ANNOTATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\((?:CI|check|enforced|ref): [^)]*\)").expect("a built-in pattern compiles")
});

static YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(19|20)\d{2}\b").expect("a built-in pattern compiles"));

pub fn check(ctx: &Ctx<'_>) -> Result<Vec<Finding>> {
    let p = &ctx.cfg.prose;
    let procedure = Paths::new(&p.procedure_paths)?;
    let exempt = Paths::new(&p.constructs_exempt)?;
    let status_exempt = Paths::new(&p.status_exempt)?;
    let mut ignore = vec![ANNOTATION.clone()];
    for i in &p.ignore {
        ignore.push(Regex::new(i).with_context(|| format!("prose.ignore pattern {i}"))?);
    }
    let mut out = Vec::new();
    for (&path, doc) in ctx.docs.iter().filter(|(p, _)| !ctx.skipped.matches(p)) {
        let all_steps = procedure.matches(path);
        for unit in &doc.units {
            let mut prose = unit.prose.clone();
            for re in &ignore {
                prose.blank(re);
            }
            let max_words = if all_steps {
                p.procedure_max_words
            } else {
                p.max_words
            };
            let opts = Options {
                english: true,
                word_slash: true,
                max_words,
                slash_terms: &p.slash_terms,
            };
            for h in simple::check(&prose.text, &opts) {
                out.push(Finding::new(path, prose.line_at(h.pos), h.rule, h.message));
            }
            if exempt.matches(path) {
                continue;
            }
            for c in constructs::all(Scope::Document) {
                if c.rule == "construct/status" && status_exempt.matches(path) {
                    continue;
                }
                for m in c.find(&prose.text) {
                    let msg = format!(
                        "{} `{}`: {}",
                        c.rule.trim_start_matches("construct/"),
                        m.as_str().trim(),
                        c.fix
                    );
                    out.push(Finding::new(path, prose.line_at(m.start()), c.rule, msg));
                }
            }
            if unit.kind == UnitKind::Heading && YEAR.is_match(&prose.text) {
                out.push(Finding::new(
                    path,
                    prose.first_line(),
                    "construct/dated-heading",
                    "write a heading with no date",
                ));
            }
        }
    }
    Ok(out)
}
