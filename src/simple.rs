//! The Simple English profile: the punctuation allow-list, the word lists, and the sentence limits. It checks one
//! prose text, which the caller builds with everything that is not prose already replaced.

use std::sync::LazyLock;

use regex::Regex;

/// A prose text and the source line of each part of it.
#[derive(Debug, Default, Clone)]
pub struct Prose {
    pub text: String,
    lines: Vec<(usize, usize)>,
}

impl Prose {
    pub fn push(&mut self, s: &str, line: usize) {
        if self.lines.last().is_none_or(|&(_, l)| l != line) {
            self.lines.push((self.text.len(), line));
        }
        self.text.push_str(s);
    }

    /// Drops the text from `pos` on, so a caller can replace the last part.
    pub fn truncate(&mut self, pos: usize) {
        self.text.truncate(pos);
        self.lines.retain(|&(p, _)| p < pos || p == 0);
    }

    pub fn line_at(&self, pos: usize) -> usize {
        let i = self.lines.partition_point(|&(p, _)| p <= pos);
        self.lines.get(i.saturating_sub(1)).map_or(0, |&(_, l)| l)
    }

    pub fn first_line(&self) -> usize {
        self.lines.first().map_or(0, |&(_, l)| l)
    }

    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// Replaces each match of a pattern with spaces, so the positions of the rest do not move.
    pub fn blank(&mut self, pattern: &Regex) {
        let spans: Vec<_> = pattern.find_iter(&self.text).map(|m| m.range()).collect();
        for r in spans {
            let blank = " ".repeat(r.len());
            self.text.replace_range(r, &blank);
        }
    }
}

pub struct Options<'a> {
    /// The word rules. A UI string in another locale gets only the punctuation and length rules.
    pub english: bool,
    /// The `A/B` rule. It is exact only where paths are in backticks, so comments skip it.
    pub word_slash: bool,
    /// Zero turns the length rule off.
    pub max_words: usize,
    pub slash_terms: &'a [String],
}

pub struct Hit {
    pub pos: usize,
    pub rule: &'static str,
    pub message: String,
}

struct Pattern {
    rule: &'static str,
    re: Regex,
    fix: &'static str,
}

fn pattern(rule: &'static str, re: &str, fix: &'static str) -> Pattern {
    Pattern {
        rule,
        re: Regex::new(re).expect("a built-in pattern compiles"),
        fix,
    }
}

// Each pattern is written so that code a text mentions does not match: `!=`, `a;b`, `&&`, and `run()` pass.
static PUNCTUATION: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    vec![
        pattern(
            "simple/semicolon",
            r#"[\p{L}\p{N})'"*_];(\s|$)"#,
            "write two sentences, or use a colon",
        ),
        pattern(
            "simple/parentheses",
            r"(^|[\s,.:])\([^)]*\)?",
            "write the aside as its own sentence, or delete it",
        ),
        pattern(
            "simple/question",
            r#"[\p{L}\p{N})'"*_]\?(\s|$)"#,
            "write the answer as a statement",
        ),
        pattern(
            "simple/exclamation",
            r#"[\p{L}\p{N})'"*_]!(\s|$)"#,
            "delete it",
        ),
        pattern("simple/ampersand", r"(^|\s)&(\s|$)", "write `and`"),
        pattern("simple/quote", r#"""#, "use backticks for literal text"),
        pattern(
            "simple/dash",
            r"\S\s--?\s\S",
            "write two sentences, or use a colon or a comma",
        ),
        pattern(
            "simple/slash",
            r"(?i)\s/\s|\band/or\b",
            "write `or`, `and`, or a list",
        ),
        pattern(
            "simple/emoji",
            r"\p{Emoji_Presentation}",
            "write the meaning in words",
        ),
    ]
});

static WORD_SLASH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(^|[\s,.:])(\p{L}+)/(\p{L}+)([\s,.:]|$)").expect("a built-in pattern compiles")
});

static ENGLISH: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    vec![
        pattern(
            "simple/contraction",
            r"(?i)\b(\p{L}+n't|it's|that's|there's|here's|what's|who's|let's|\p{L}+'re|\p{L}+'ve|\p{L}+'ll|i'm)\b",
            "write the full form: `do not`, `it is`",
        ),
        pattern(
            "simple/latin",
            r"(?i)(\be\.g\.|\bi\.e\.|\betc\.|\bviz\.|\bcf\.|\bvs\.?(\s|$))",
            "write `for example`, `that is`, a complete list, or `or`",
        ),
    ]
});

/// A word above CEFR B1, as a pattern, and its simple form.
const COMPLEX_WORDS: &[(&str, &str)] = &[
    (r"utili[sz](e|es|ed|ing|ation)", "use"),
    (r"leverag(e|es|ed|ing)", "use"),
    (r"facilitat(e|es|ed|ing)", "help"),
    (r"prior to", "before"),
    (r"in order to", "to"),
    (r"subsequently", "then, or later"),
    (r"hence", "so"),
    (r"thus", "so"),
    (r"thereby", "so"),
    (r"whereby", "by which, or a new sentence"),
    (r"wherein", "where"),
    (r"whilst", "while"),
    (r"amongst", "among"),
    (r"albeit", "although"),
    (r"notwithstanding", "despite"),
    (r"aforementioned", "this, or the name"),
    (r"commenc(e|es|ed|ing)", "start"),
    (r"endeavou?r", "try"),
    (r"in lieu of", "instead of"),
    (r"moreover", "also"),
    (r"furthermore", "also"),
    (r"nevertheless", "but"),
    (r"nonetheless", "still"),
    (r"whereas", "but, or a new sentence"),
    (r"henceforth", "from now"),
    (r"insofar as", "if, or to the degree that"),
    (r"inasmuch as", "because"),
    (r"vis-à-vis", "about, or compared with"),
    (r"per se", "nothing: delete it"),
    (r"ergo", "so"),
    (r"ascertain(s|ed|ing)?", "find, or check"),
    (r"elucidat(e|es|ed|ing)", "explain"),
    (r"obviat(e|es|ed|ing)", "remove the need for"),
    (r"necessitat(e|es|ed|ing)", "need"),
    (r"pertaining to", "about"),
    (r"in the event that", "if"),
    (r"with (respect|regard) to", "about"),
    (r"the majority of", "most"),
    (r"is able to", "can"),
    (r"via", "through, by, or with"),
];

static COMPLEX: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    COMPLEX_WORDS
        .iter()
        .map(|(w, simple)| {
            (
                Regex::new(&format!(r"(?i)\b({w})\b")).expect("a built-in pattern compiles"),
                *simple,
            )
        })
        .collect()
});

/// A sentence ends at a period, colon, or question or exclamation mark before a space or the end. A colon ends a
/// unit too: it introduces a list or an explanation. A period inside `3.1` or a file name is not an end.
static SENTENCE_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.:?!][*_]*(\s+|$)").expect("a built-in pattern compiles"));

static WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\p{L}\p{N}]").expect("a built-in pattern compiles"));

pub fn check(text: &str, o: &Options<'_>) -> Vec<Hit> {
    let mut hits = Vec::new();
    for p in PUNCTUATION.iter() {
        for m in p.re.find_iter(text) {
            hits.push(hit(
                m.start(),
                p.rule,
                format!("{} `{}`: {}", label(p.rule), m.as_str().trim(), p.fix),
            ));
        }
    }
    if o.word_slash {
        for c in WORD_SLASH.captures_iter(text) {
            let pair = format!("{}/{}", &c[2], &c[3]);
            // `and/or` is the punctuation rule's.
            if !pair.eq_ignore_ascii_case("and/or")
                && !o.slash_terms.iter().any(|t| t.eq_ignore_ascii_case(&pair))
            {
                let pos = c.get(2).map_or(0, |m| m.start());
                hits.push(hit(
                    pos,
                    "simple/slash",
                    format!(
                        "slash `{pair}`: write `or`, `and`, or a list. Put a path in backticks"
                    ),
                ));
            }
        }
    }
    if o.english {
        for p in ENGLISH.iter() {
            for m in p.re.find_iter(text) {
                hits.push(hit(
                    m.start(),
                    p.rule,
                    format!("{} `{}`: {}", label(p.rule), m.as_str().trim(), p.fix),
                ));
            }
        }
        for (re, simple) in COMPLEX.iter() {
            for m in re.find_iter(text) {
                hits.push(hit(
                    m.start(),
                    "simple/complex-word",
                    format!("complex word `{}`: write `{simple}`", m.as_str()),
                ));
            }
        }
    }
    if o.max_words > 0 {
        let mut start = 0;
        for end in SENTENCE_END
            .find_iter(text)
            .map(|m| (m.start(), m.end()))
            .chain(std::iter::once((text.len(), text.len())))
        {
            let sentence = &text[start..end.0];
            let words = words(sentence);
            if words > o.max_words {
                let at = start + (sentence.len() - sentence.trim_start().len());
                hits.push(hit(
                    at,
                    "simple/sentence-length",
                    format!(
                        "long sentence `{}`: {words} words, max {}. Split it",
                        clip(sentence),
                        o.max_words
                    ),
                ));
            }
            start = end.1;
        }
    }
    hits
}

/// Counts the tokens that hold a letter or a digit.
pub fn words(sentence: &str) -> usize {
    sentence
        .split_whitespace()
        .filter(|t| WORD.is_match(t))
        .count()
}

fn hit(pos: usize, rule: &'static str, message: String) -> Hit {
    Hit { pos, rule, message }
}

fn label(rule: &str) -> String {
    rule.trim_start_matches("simple/").replace('-', " ")
}

fn clip(s: &str) -> String {
    let words: Vec<_> = s.split_whitespace().collect();
    if words.len() > 8 {
        format!("{} ...", words[..8].join(" "))
    } else {
        words.join(" ")
    }
}

#[cfg(test)]
#[allow(clippy::assert_is_empty)]
mod tests {
    use super::*;

    fn rules(text: &str) -> Vec<&'static str> {
        let terms = vec!["ci/cd".to_owned()];
        let o = Options {
            english: true,
            word_slash: true,
            max_words: 25,
            slash_terms: &terms,
        };
        let mut r: Vec<_> = check(text, &o).into_iter().map(|h| h.rule).collect();
        r.sort_unstable();
        r.dedup();
        r
    }

    // The Bad and Good columns of the Words table in ADR-0001.
    #[test]
    fn words_table() {
        for bad in [
            "We utilize it.",
            "We leverage it.",
            "It will facilitate it.",
            "Do it prior to that.",
            "We do it in order to win.",
        ] {
            assert_eq!(rules(bad), ["simple/complex-word"], "{bad}");
        }
        assert_eq!(rules("Use it, e.g. here."), ["simple/latin"]);
        assert_eq!(rules("Don't do it. It's here."), ["simple/contraction"]);
        assert!(rules("We use it before the build, to help.").is_empty());
    }

    #[test]
    fn punctuation_table() {
        assert_eq!(rules("One clause; two clauses."), ["simple/semicolon"]);
        assert_eq!(rules("An aside (like this) here."), ["simple/parentheses"]);
        assert_eq!(rules("Is it ready?"), ["simple/question"]);
        assert_eq!(rules("Done!"), ["simple/exclamation"]);
        assert_eq!(rules("Cats & dogs."), ["simple/ampersand"]);
        assert_eq!(rules("It said \"hi\"."), ["simple/quote"]);
        assert_eq!(rules("A - B."), ["simple/dash"]);
        assert_eq!(rules("Use and/or here."), ["simple/slash"]);
        assert_eq!(rules("Pick red/blue."), ["simple/slash"]);
        assert_eq!(rules("Ship it \u{1F680}."), ["simple/emoji"]);
    }

    #[test]
    fn code_shapes_pass() {
        assert!(rules("Call run() when a != b and x&&y, then CI/CD runs.").is_empty());
        assert!(rules("Version 3.1 of the file.txt is the floor.").is_empty());
    }

    #[test]
    fn sentence_length() {
        let long = "word ".repeat(26);
        assert_eq!(
            rules(&format!("{}.", long.trim())),
            ["simple/sentence-length"]
        );
        assert!(
            rules(&format!(
                "{}: {}.",
                "word ".repeat(20).trim(),
                "word ".repeat(20).trim()
            ))
            .is_empty()
        );
        assert_eq!(words("a `x` b-c 3.1 -- !"), 4);
    }

    #[test]
    fn prose_maps_positions_to_lines() {
        let mut p = Prose::default();
        p.push("first ", 3);
        p.push("second", 4);
        assert_eq!(p.line_at(0), 3);
        assert_eq!(p.line_at(7), 4);
        p.truncate(6);
        p.push("x", 5);
        assert_eq!(p.text, "first x");
        assert_eq!(p.line_at(6), 5);
    }
}
