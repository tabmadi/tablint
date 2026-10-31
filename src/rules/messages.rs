//! UI strings, one JSON catalogue per locale. The source locale gets the whole profile, and every other locale gets
//! the punctuation and length rules, which hold in any language.

use std::sync::LazyLock;

use anyhow::{Context, Result};
use regex::Regex;
use serde_json::Value;

use super::Ctx;
use crate::finding::Finding;
use crate::simple::{self, Options};

/// An ICU argument or plural block is syntax, not prose.
static ICU: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{[^{}]*\}").expect("a built-in pattern compiles"));

pub fn check(ctx: &Ctx<'_>) -> Result<Vec<Finding>> {
    let Some(dir) = ctx.cfg.messages.dir.as_deref() else {
        return Ok(Vec::new());
    };
    let dir = dir.trim_end_matches('/');
    let mut out = Vec::new();
    for path in ctx.files().filter(|f| {
        f.rsplit_once('/')
            .is_some_and(|(d, n)| d == dir && n.ends_with(".json"))
    }) {
        let Some(text) = ctx.repo.read(path) else {
            continue;
        };
        let tree: Value = serde_json::from_str(&text).with_context(|| format!("parse {path}"))?;
        let locale = path
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .trim_end_matches(".json");
        let opts = Options {
            english: locale == ctx.cfg.messages.source_locale,
            word_slash: false,
            max_words: ctx.cfg.prose.max_words,
            slash_terms: &[],
        };
        let mut strings = Vec::new();
        walk(&tree, String::new(), &mut strings);
        for (key, value) in strings {
            let line = line_of(&text, &key);
            for h in simple::check(&ICU.replace_all(&value, "x"), &opts) {
                out.push(Finding::new(
                    path,
                    line,
                    h.rule,
                    format!("{key}: {}", h.message),
                ));
            }
        }
    }
    Ok(out)
}

fn walk(v: &Value, key: String, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(map) => {
            for (k, child) in map {
                walk(
                    child,
                    if key.is_empty() {
                        k.clone()
                    } else {
                        format!("{key}.{k}")
                    },
                    out,
                );
            }
        }
        Value::String(s) => out.push((key, s.clone())),
        _ => {}
    }
}

/// The line of a key's last part. A key that repeats under another parent reports the first one.
fn line_of(text: &str, key: &str) -> usize {
    let last = format!("\"{}\"", key.rsplit('.').next().unwrap_or(key));
    text.lines()
        .position(|l| l.trim_start().starts_with(&last))
        .map_or(0, |i| i + 1)
}
