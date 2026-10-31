//! The characters ADR-0001 bans from every tracked text file, code included. A copy from prose is the only way they
//! arrive, and they have no use in code.

use super::Ctx;
use crate::finding::Finding;

const BANNED: &[(char, &str, &str)] = &[
    (
        '\u{2014}',
        "em dash",
        "write two sentences, or use a colon or a comma",
    ),
    ('\u{2013}', "en dash", "write `1 to 5` for a range"),
    ('\u{2026}', "ellipsis", "complete the list, or delete it"),
    ('\u{201c}', "curly quote", "use backticks for literal text"),
    ('\u{201d}', "curly quote", "use backticks for literal text"),
    ('\u{2018}', "curly quote", "use backticks for literal text"),
    ('\u{2019}', "curly quote", "use backticks for literal text"),
    ('\u{201e}', "low quote", "use backticks for literal text"),
    ('\u{00ab}', "guillemet", "use backticks for literal text"),
    ('\u{00bb}', "guillemet", "use backticks for literal text"),
    (
        '\u{061f}',
        "Arabic question mark",
        "write the answer as a statement",
    ),
    ('\u{061b}', "Arabic semicolon", "write two sentences"),
];

pub fn check(ctx: &Ctx<'_>) -> Vec<Finding> {
    let mut out = Vec::new();
    for path in ctx.files() {
        let Some(text) = ctx.repo.read(path) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            if let Some((_, name, fix)) = line
                .chars()
                .find_map(|c| BANNED.iter().find(|(b, _, _)| *b == c))
            {
                out.push(Finding::new(
                    path,
                    i + 1,
                    "char/banned",
                    format!("{name}: {fix}"),
                ));
            }
        }
    }
    out
}
