//! The comments of a source file, grouped into blocks. A run of own-line comments is one block, and a comment after
//! code on its line is a block of its own.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// Go, TypeScript, JavaScript, and Rust: `//` and `/* */`.
    CFamily,
    Rust,
    /// Shell, Python, GDScript, TOML, and other `#` languages.
    Hash,
    /// YAML: `#`, and the `{{/* */}}` comments of a Helm template.
    Yaml,
    /// SQL: `--`.
    Sql,
}

impl Lang {
    pub fn of(path: &str) -> Option<Self> {
        let p = Path::new(path);
        let name = p.file_name()?.to_str()?;
        if matches!(name, "Dockerfile" | "Makefile" | "justfile") || name.ends_with(".dockerfile") {
            return Some(Self::Hash);
        }
        match p.extension()?.to_str()? {
            "go" | "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => Some(Self::CFamily),
            "rs" => Some(Self::Rust),
            "sh" | "bash" | "zsh" | "toml" | "py" | "gd" | "rb" | "tf" | "hcl" => Some(Self::Hash),
            "yaml" | "yml" => Some(Self::Yaml),
            "sql" => Some(Self::Sql),
            _ => None,
        }
    }

    fn line_mark(self) -> &'static str {
        match self {
            Self::CFamily | Self::Rust => "//",
            Self::Hash | Self::Yaml => "#",
            Self::Sql => "--",
        }
    }

    fn block_marks(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::CFamily | Self::Rust => Some(("/*", "*/")),
            Self::Yaml => Some(("{{/*", "*/}}")),
            Self::Hash | Self::Sql => None,
        }
    }

    pub fn is_sql(self) -> bool {
        self == Self::Sql
    }
}

#[derive(Debug, Default, Clone)]
pub struct Block {
    /// The source line of the first comment line.
    pub start: usize,
    pub lines: Vec<String>,
    /// The comment text of each line, without its marker.
    pub bodies: Vec<String>,
    pub trailing: bool,
    /// The first code line under the block.
    pub next: String,
}

/// A comment a tool reads. Deleting one changes behaviour, so no rule applies to it.
static DIRECTIVE: LazyLock<Regex> = LazyLock::new(|| {
    re(concat!(
        r"(?i)^\s*(shellcheck\s|renovate:|yaml-language-server:|SPDX-|nolint|eslint-|biome-ignore|@?ts-|prettier-|",
        r"sqlc:|go:generate|go:build|go:embed|\+build|noqa|type:\s|pragma|pylint:|gdlint:|fmt:|clippy::|rustfmt::|",
        r"tablint:allow|lint:comments-allow|name:\s\w+\s:|migrate:|/\s*<reference\s|istanbul |c8 |cspell:|vale )"
    ))
});

static HEREDOC: LazyLock<Regex> =
    LazyLock::new(|| re(r#"<<-?\s*['"]?([A-Za-z_][A-Za-z0-9_]*)['"]?"#));

fn re(p: &str) -> Regex {
    Regex::new(p).expect("a built-in pattern compiles")
}

/// Returns the comment blocks of a file and the count of its comment lines. `extra` holds the directive prefixes a
/// repository adds, and `regions` holds marker pairs around generated data.
pub fn blocks(
    lang: Lang,
    text: &str,
    extra: &[String],
    regions: &[[String; 2]],
) -> (Vec<Block>, usize) {
    let lines: Vec<&str> = text.lines().collect();
    let data = data_lines(lang, &lines, regions);
    let mut out = Vec::new();
    let mut count = 0;
    let mut run: Option<Block> = None;
    let mut in_block = false;
    let flush = |run: &mut Option<Block>, out: &mut Vec<Block>, next: &str| {
        if let Some(mut b) = run.take() {
            next.clone_into(&mut b.next);
            out.push(b);
        }
    };
    for (i, line) in lines.iter().enumerate() {
        if data[i] {
            flush(&mut run, &mut out, line);
            continue;
        }
        let Some((body, own)) = comment_body(lang, line, &mut in_block) else {
            flush(&mut run, &mut out, line);
            continue;
        };
        if DIRECTIVE.is_match(&body)
            || extra
                .iter()
                .any(|d| body.trim_start().starts_with(d.as_str()))
        {
            flush(&mut run, &mut out, line);
            continue;
        }
        count += 1;
        if !own {
            flush(&mut run, &mut out, "");
            out.push(Block {
                start: i + 1,
                lines: vec![(*line).to_owned()],
                bodies: vec![body],
                trailing: true,
                next: String::new(),
            });
            continue;
        }
        let b = run.get_or_insert_with(|| Block {
            start: i + 1,
            ..Block::default()
        });
        b.lines.push((*line).to_owned());
        b.bodies.push(body);
        // A closing `*/` ends the block it opened: two adjacent doc comments are two comments, not one run.
        if !in_block
            && lang
                .block_marks()
                .is_some_and(|(_, close)| line.trim().ends_with(close))
        {
            flush(&mut run, &mut out, "");
        }
    }
    flush(&mut run, &mut out, "");
    (out, count)
}

/// Returns a line's comment text and whether the comment owns the line, or `None` for a line with no comment.
fn comment_body(lang: Lang, line: &str, in_block: &mut bool) -> Option<(String, bool)> {
    let trimmed = line.trim();
    if *in_block {
        let (_, close) = lang.block_marks()?;
        if trimmed.contains(close) {
            *in_block = false;
        }
        return Some((
            trimmed
                .trim_end_matches(close)
                .trim_start_matches('*')
                .to_owned(),
            true,
        ));
    }
    if let Some((open, close)) = lang.block_marks()
        && let Some(rest) = trimmed.strip_prefix(open)
    {
        *in_block = !rest.contains(close);
        return Some((
            rest.trim_end_matches(close)
                .trim_start_matches('*')
                .to_owned(),
            true,
        ));
    }
    if trimmed.starts_with("#!") {
        return None;
    }
    let mark = lang.line_mark();
    if let Some(rest) = trimmed.strip_prefix(mark) {
        // A Rust doc comment is `///` or `//!`, and a GDScript doc comment is `##`.
        let rest = match lang {
            Lang::Rust => rest.trim_start_matches(['/', '!']),
            Lang::Hash => rest.trim_start_matches('#'),
            _ => rest,
        };
        return Some((rest.to_owned(), true));
    }
    let idx = trailing_mark(trimmed, mark)?;
    Some((trimmed[idx + mark.len()..].to_owned(), false))
}

/// The position of a comment mark after code, or `None`. A `//` after a colon is a URL scheme, a `#` inside a word
/// is not a comment in shell or YAML, and a mark inside quotes is data.
fn trailing_mark(line: &str, mark: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(found) = line[from..].find(mark) {
        let idx = from + found;
        from = idx + mark.len();
        let before = &line[..idx];
        let prev = before.chars().next_back();
        if mark == "//" && prev == Some(':') {
            continue;
        }
        if mark == "#" && prev.is_some_and(|c| !c.is_whitespace()) {
            continue;
        }
        if before.matches('"').count() % 2 == 1
            || before.matches('\'').count() % 2 == 1
            || before.matches('`').count() % 2 == 1
        {
            continue;
        }
        return Some(idx);
    }
    None
}

/// Marks the lines that are data, not comments: a string literal across lines, a shell heredoc, and a region a
/// generator writes. Deleting a comment marker inside one changes what the program emits.
fn data_lines(lang: Lang, lines: &[&str], regions: &[[String; 2]]) -> Vec<bool> {
    let mut data = vec![false; lines.len()];
    let mut region: Option<&str> = None;
    let mut heredoc: Option<String> = None;
    let mut quote: Option<&str> = None;
    for (i, line) in lines.iter().enumerate() {
        if let Some(end) = region {
            data[i] = true;
            if line.contains(end) {
                region = None;
            }
            continue;
        }
        if let Some([_, end]) = regions
            .iter()
            .find(|[start, _]| line.contains(start.as_str()))
        {
            data[i] = true;
            region = Some(end.as_str());
            continue;
        }
        if let Some(term) = &heredoc {
            data[i] = true;
            if line.trim() == term {
                heredoc = None;
            }
            continue;
        }
        if let Some(q) = quote {
            data[i] = true;
            if line.matches(q).count() % 2 == 1 {
                quote = None;
            }
            continue;
        }
        if lang == Lang::Hash
            && let Some(c) = HEREDOC.captures(line)
        {
            heredoc = Some(c[1].to_owned());
            continue;
        }
        // A string that stays open at the end of the line: a Go or JavaScript template, a Python or GDScript
        // docstring, or a Rust raw string.
        for q in multiline_quotes(lang) {
            if line.matches(q).count() % 2 == 1 && !line.trim_start().starts_with(lang.line_mark())
            {
                quote = Some(q);
                break;
            }
        }
    }
    data
}

fn multiline_quotes(lang: Lang) -> &'static [&'static str] {
    match lang {
        Lang::CFamily => &["`"],
        Lang::Rust => &["\"#"],
        Lang::Hash => &["\"\"\"", "'''"],
        Lang::Yaml | Lang::Sql => &[],
    }
}

#[cfg(test)]
#[allow(clippy::assert_is_empty)]
mod tests {
    use super::*;

    fn bodies(lang: Lang, text: &str) -> Vec<(usize, Vec<String>, bool)> {
        blocks(lang, text, &[], &[])
            .0
            .into_iter()
            .map(|b| (b.start, b.bodies, b.trailing))
            .collect()
    }

    #[test]
    fn groups_runs_and_trailing_comments() {
        let go = "// One.\n// Two.\nx := 1 // Three.\nurl := \"http://a\"\n/* Four. */\n";
        let got = bodies(Lang::CFamily, go);
        assert_eq!(got[0], (1, vec![" One.".into(), " Two.".into()], false));
        assert_eq!(got[1], (3, vec![" Three.".into()], true));
        assert_eq!(got[2].0, 5);
        assert_eq!(got.len(), 3);
    }

    #[test]
    fn skips_directives_data_and_shell_words() {
        let sh = "#!/bin/sh\n# shellcheck disable=SC2086\necho \"${#x}\" # Count.\ncat <<EOF\n# not a comment\nEOF\n";
        let got = bodies(Lang::Hash, sh);
        assert_eq!(got, [(3, vec![" Count.".into()], true)]);
        let py = "def f():\n    \"\"\"Doc.\n    # not a comment\n    \"\"\"\n";
        assert!(bodies(Lang::Hash, py).is_empty());
    }

    #[test]
    fn rust_doc_comments() {
        let got = bodies(Lang::Rust, "/// Returns the count.\npub fn n() {}\n");
        assert_eq!(got[0].1, [" Returns the count."]);
        let (b, _) = blocks(
            Lang::Rust,
            "/// Returns the count.\npub fn n() {}\n",
            &[],
            &[],
        );
        assert_eq!(b[0].next, "pub fn n() {}");
    }

    #[test]
    fn regions_from_config() {
        let regions = [[">>> gen".to_owned(), "<<< gen".to_owned()]];
        let (b, _) = blocks(
            Lang::Yaml,
            "# >>> gen\n# data\n# <<< gen\n# Real.\n",
            &[],
            &regions,
        );
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].start, 4);
    }
}
