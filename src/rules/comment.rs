//! The comment rules of ADR-0001: the three tests a machine can check, the banned comment shapes, and the Simple
//! English profile over the prose of each block.

use std::sync::LazyLock;

use anyhow::Result;
use regex::Regex;

use super::Ctx;
use crate::comments::{self, Block, Lang};
use crate::config::Paths;
use crate::constructs::{self, Scope};
use crate::finding::Finding;
use crate::simple::{self, Options};

/// A doc comment longer than this carries content that the echo test cannot recognise.
const MAX_ECHO_CHARS: usize = 90;

fn re(p: &str) -> Regex {
    Regex::new(p).expect("a built-in pattern compiles")
}

static SHAPES: LazyLock<Vec<(&'static str, Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        (
            "comment/changelog",
            re(
                r"(?i)(^|\s)(@author|@since|@deprecated\s+\d|author:|created:|last (modified|updated):)",
            ),
            "git holds authorship and dates",
        ),
        (
            "comment/banner",
            re(r"([=*#_~-]|─|━|═){8,}"),
            "the file structure is the structure",
        ),
    ]
});

static TODO: LazyLock<Regex> = LazyLock::new(|| re(r"\b(TODO|FIXME)\b"));
static CITATION: LazyLock<Regex> = LazyLock::new(|| re(r"(ADR-\d{4}|#\d+)"));

// A comment whose body is code. Conservative on purpose: a false positive lands on a comment that does its job.
static CODEISH: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^\s*(if|for|while|switch|return|func|function|const|let|var|import|export|class|type|await|go |fn|pub|use|impl|match)\b.*[;{}()]\s*$",
    )
});
// A bare assignment or call. A call admits no space before its paren, which keeps prose with an aside out.
static ASSIGNMENTISH: LazyLock<Regex> =
    LazyLock::new(|| re(r"^\s*[\w.\[\]]+(\s*(?::?=|\+=)|\()[^)]*[);]\s*$"));
// A Python or GDScript definition.
static DEFINITION: LazyLock<Regex> = LazyLock::new(|| re(r"^\s*(def|func)\s+\w+\(.*\).*:\s*$"));

static EXPORTED: LazyLock<Regex> = LazyLock::new(|| {
    re(concat!(
        r"^\s*(?:export\s+(?:default\s+)?)?(?:func|type|const|var|let|class|interface|function|enum)\s+",
        r"(?:\(\w+\s+\*?\w+\)\s+)?([A-Z]\w*)",
        r"|^\s*pub(?:\([^)]*\))?\s+(?:async\s+)?(?:unsafe\s+)?(?:fn|struct|enum|trait|type|const|static|mod|union)\s+(\w+)"
    ))
});

// A fact a signature cannot state. A doc comment on an exported identifier carries one, or it restates the
// declaration and is deleted.
static SIGNAL: LazyLock<Regex> = LazyLock::new(|| {
    re(concat!(
        r"(?i)(ADR-\d{4}|(?-i:\b[A-Z]{2,4}-\d{3}\b)|",
        r"\bnil\b|\bnone\b|\bnull\b|\bmust\b|\bnever\b|\bonly\b|\bpanic|\berror|\bcaller|\bzero\b|\bempty\b|\bnegative\b|",
        r"\binclusive\b|\bexclusive\b|\bat least\b|\bat most\b|\bconcurrent|\bblocks\b|\bcloses\b|\bowns\b|\bmutates\b|",
        r"\bretains\b|\bunsafe\b|\bUTC\b|\bcents\b|\bbytes\b|\bseconds\b|\bdoes not\b|\bcannot\b|\bis not\b|\bare not\b|",
        r"\bno\b|\bnot\b|\$[A-Z_]{3,}|\bdefault|\bRFC \d|\bcolumn\b|\bwire\b|\bimplements\b|\bISO \d|\bsorted\b|\bunique\b)"
    ))
});

static DOC_CODE: LazyLock<Regex> = LazyLock::new(|| re(r"^(\t|\s{3,})"));
static CODE_SPAN: LazyLock<Regex> = LazyLock::new(|| re("`[^`]*`"));
static URL: LazyLock<Regex> = LazyLock::new(|| re(r"<?https?://[^\s)>]+>?"));
static ANNOTATION: LazyLock<Regex> =
    LazyLock::new(|| re(r"`?\((CI|check|enforced|ref): [^)]*\)`?"));
static CALL: LazyLock<Regex> = LazyLock::new(|| re(r"([\p{L}\p{N}_])\([^)\s]*\)"));

pub fn check(ctx: &Ctx<'_>) -> Result<Vec<Finding>> {
    let c = &ctx.cfg.comments;
    let scope = Paths::new(&c.paths)?;
    let mut out = Vec::new();
    for path in ctx
        .files()
        .filter(|p| c.paths.is_empty() || scope.matches(p))
    {
        let Some(lang) = Lang::of(path) else { continue };
        let generated = path
            .split('/')
            .any(|part| c.skip_dirs.iter().any(|d| d == part))
            || c.skip_suffixes.iter().any(|s| path.ends_with(s.as_str()));
        if generated {
            continue;
        }
        let Some(text) = ctx.repo.read(path) else {
            continue;
        };
        let (blocks, _) = comments::blocks(lang, &text, &c.directives, &c.skip_regions);
        for b in &blocks {
            if !lang.is_sql() {
                out.extend(shapes(path, b, c.max_lines));
            }
            out.extend(profile(ctx, path, b));
        }
    }
    Ok(out)
}

fn shapes(path: &str, b: &Block, max_lines: usize) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut code = false;
    for (i, body) in b.bodies.iter().enumerate() {
        let line = b.start + i;
        if body.trim_start().starts_with("```") {
            code = !code;
            continue;
        }
        if code {
            continue;
        }
        for c in constructs::all(Scope::Comment) {
            if let Some(m) = c.find(body).next() {
                let msg = format!(
                    "{} `{}`: {}",
                    c.rule.trim_start_matches("construct/"),
                    m.as_str().trim(),
                    c.fix
                );
                out.push(Finding::new(path, line, c.rule, msg));
            }
        }
        for (rule, re, fix) in SHAPES.iter() {
            if re.is_match(body) {
                out.push(Finding::new(path, line, rule, *fix));
            }
        }
        if TODO.is_match(body) && !CITATION.is_match(body) {
            out.push(Finding::new(
                path,
                line,
                "comment/todo",
                "cite an ADR such as `ADR-0123` or an issue such as `#123`",
            ));
        }
        // A tab-indented line inside a doc comment is a rendered code block, not a call someone commented out.
        let codelike =
            CODEISH.is_match(body) || ASSIGNMENTISH.is_match(body) || DEFINITION.is_match(body);
        if !body.starts_with('\t') && !CITATION.is_match(body) && codelike {
            out.push(Finding::new(
                path,
                line,
                "comment/commented-out-code",
                "git holds it. Delete it",
            ));
        }
    }
    if b.trailing {
        return out;
    }
    if paragraphs(&b.bodies) > 1 {
        out.push(Finding::new(
            path,
            b.start,
            "comment/paragraphs",
            "a second paragraph is a document. Move it to docs/ and cite it",
        ));
    }
    let n = b.bodies.iter().filter(|body| !blank(body)).count();
    if n > max_lines {
        out.push(Finding::new(
            path,
            b.start,
            "comment/length",
            format!("{n} lines, max {max_lines}. One line is the norm"),
        ));
    }
    if let Some(name) = EXPORTED
        .captures(&b.next)
        .and_then(|c| c.get(1).or_else(|| c.get(2)))
        && b.bodies.len() == 1
        && b.bodies[0].trim().len() < MAX_ECHO_CHARS
        && !SIGNAL.is_match(&b.bodies[0])
        && echoes(&b.bodies[0], name.as_str())
    {
        let msg = format!(
            "state a fact that the signature of `{}` cannot: units, empty values, bounds, side effects, or errors. Otherwise delete it",
            name.as_str()
        );
        out.push(Finding::new(path, b.start, "comment/echo-doc", msg));
    }
    out
}

/// The Simple English profile over the block's prose. A sentence can span lines, so the block is joined first.
fn profile(ctx: &Ctx<'_>, path: &str, b: &Block) -> Vec<Finding> {
    let mut prose = Vec::new();
    let mut code = false;
    for body in &b.bodies {
        if body.trim_start().starts_with("```") {
            code = !code;
            continue;
        }
        if code || DOC_CODE.is_match(body) || blank(body) {
            continue;
        }
        prose.push(body.trim().trim_start_matches('*').trim());
    }
    let text = prose.join(" ");
    let text = ANNOTATION.replace_all(&text, "");
    let text = CODE_SPAN.replace_all(&text, "x");
    let text = URL.replace_all(&text, "x");
    let text = CALL.replace_all(&text, "$1");
    let opts = Options {
        english: true,
        word_slash: false,
        max_words: ctx.cfg.prose.max_words,
        slash_terms: &[],
    };
    simple::check(&text, &opts)
        .into_iter()
        .map(|h| Finding::new(path, b.start, h.rule, h.message))
        .collect()
}

/// Words that carry no fact in a doc comment: articles, and the verbs every declaration implies.
const FILLER: &[&str] = &[
    "a",
    "an",
    "the",
    "is",
    "are",
    "of",
    "for",
    "to",
    "this",
    "that",
    "it",
    "and",
    "returns",
    "return",
    "creates",
    "create",
    "gets",
    "get",
    "sets",
    "set",
    "new",
    "value",
    "type",
    "function",
    "method",
    "struct",
    "constant",
    "field",
    "holds",
    "represents",
    "defines",
];

/// Whether a doc comment only restates its identifier: every word is a word of the name or filler.
fn echoes(comment: &str, name: &str) -> bool {
    let mut parts: Vec<String> = vec![name.to_lowercase()];
    let mut word = String::new();
    for c in name.chars() {
        if (c.is_uppercase() || c == '_') && !word.is_empty() {
            parts.push(std::mem::take(&mut word).to_lowercase());
        }
        if c != '_' {
            word.push(c);
        }
    }
    parts.push(word.to_lowercase());
    comment
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .all(|w| {
            FILLER.contains(&w.as_str())
                || parts
                    .iter()
                    .any(|p| !p.is_empty() && (w == *p || w.trim_end_matches('s') == p))
        })
}

/// Counts the paragraphs of a block. A blank line at either end separates the prose from a directive, not one
/// paragraph from another.
fn paragraphs(bodies: &[String]) -> usize {
    let first = bodies.iter().position(|b| !blank(b));
    let last = bodies.iter().rposition(|b| !blank(b));
    match (first, last) {
        (Some(f), Some(l)) => 1 + bodies[f..=l].iter().filter(|b| blank(b)).count(),
        _ => 0,
    }
}

fn blank(body: &str) -> bool {
    body.trim().trim_matches(['*', '/']).trim().is_empty()
}

#[cfg(test)]
#[allow(clippy::assert_is_empty)]
mod tests {
    use super::*;

    #[test]
    fn echo_is_a_restated_name() {
        assert!(echoes("NewServer creates a new server.", "NewServer"));
        assert!(echoes("The config.", "Config"));
        assert!(!echoes(
            "The share of the screen height that the panel takes.",
            "HEIGHT"
        ));
        assert!(!echoes(
            "Counts the tokens that hold a letter or a digit.",
            "words"
        ));
    }

    #[test]
    fn paragraphs_ignore_blank_ends() {
        let b = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(paragraphs(&b(&["", " One.", "", " Two.", ""])), 2);
        assert_eq!(paragraphs(&b(&[" One.", " More."])), 1);
    }
}
