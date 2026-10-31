//! The banned constructs of ADR-0001: words and shapes that make a text weaker, not wrong. A document and a comment
//! get different lists, because a comment names code and a document does not.

use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Document,
    Comment,
}

pub struct Construct {
    pub rule: &'static str,
    re: Regex,
    pub fix: &'static str,
    scope: Scope,
}

impl Construct {
    pub fn find<'t>(&self, text: &'t str) -> impl Iterator<Item = regex::Match<'t>> {
        self.re.find_iter(text)
    }
}

fn c(rule: &'static str, scope: Scope, re: &str, fix: &'static str) -> Construct {
    Construct {
        rule,
        re: Regex::new(re).expect("a built-in pattern compiles"),
        fix,
        scope,
    }
}

static CONSTRUCTS: LazyLock<Vec<Construct>> = LazyLock::new(|| {
    use Scope::{Comment, Document};
    let intensifier = "delete it. A claim that needs an intensifier is not established";
    let hedge = "decide. A hedge is an unfinished decision";
    vec![
        c(
            "construct/intensifier",
            Document,
            r"(?i)\b(very|really|quite|actually|simply|just|obviously|clearly|of course)\b",
            intensifier,
        ),
        c(
            "construct/intensifier",
            Comment,
            r"(?i)\b(very|really|quite|obviously|of course)\b",
            intensifier,
        ),
        c(
            "construct/hedge",
            Document,
            r"(?i)\b(arguably|essentially|basically|somewhat|fairly|more or less|in practice)\b",
            hedge,
        ),
        c(
            "construct/hedge",
            Comment,
            r"(?i)\b(arguably|essentially|basically|somewhat)\b",
            hedge,
        ),
        c(
            "construct/meta-commentary",
            Document,
            r"(?i)(note that|it should be noted|it is worth|worth noting|worth knowing|as mentioned|as noted above)",
            "state the thing, without an introduction",
        ),
        c(
            "construct/meta-commentary",
            Comment,
            r"(?i)(it is worth (knowing|noting)|it should be noted|note that,)",
            "state the thing",
        ),
        c(
            "construct/status",
            Document,
            r"(?i)\b(not yet|so far|for now|at present|currently|the status quo|tracked in|lands in a later)\b",
            "state the rule with no qualifier. Whether the artefact exists is not the subject",
        ),
        c(
            "construct/chronology",
            Document,
            r"(?i)(\bhas since\b|\bused to be\b|\bit turned out\b|\bpreviously,|\bdid not previously\b|\bin earlier versions\b)",
            "state the standing fact: what is true now. Git holds the history",
        ),
        c(
            "construct/chronology",
            Comment,
            r"(?i)\b(used to be|previously (was|did|had)|has since|we then (added|changed)|originally (this|it) )",
            "state what it is now",
        ),
        c(
            "construct/planned-work",
            Document,
            r"(^|\s)(TODO|FIXME|Follow-ups?:)",
            "a task is an issue, not a line in a committed file",
        ),
    ]
});

pub fn all(scope: Scope) -> impl Iterator<Item = &'static Construct> {
    CONSTRUCTS.iter().filter(move |c| c.scope == scope)
}

#[cfg(test)]
#[allow(clippy::assert_is_empty)]
mod tests {
    use super::*;

    fn rules(scope: Scope, text: &str) -> Vec<&'static str> {
        all(scope)
            .filter(|c| c.find(text).next().is_some())
            .map(|c| c.rule)
            .collect()
    }

    // The Example column of the banned-constructs table in ADR-0001.
    #[test]
    fn constructs_table() {
        assert_eq!(
            rules(Scope::Document, "It is very simple."),
            ["construct/intensifier"]
        );
        assert_eq!(
            rules(Scope::Document, "It is basically done."),
            ["construct/hedge"]
        );
        assert_eq!(
            rules(Scope::Document, "Note that it matters."),
            ["construct/meta-commentary"]
        );
        assert_eq!(
            rules(Scope::Document, "It is not yet wired."),
            ["construct/status"]
        );
        assert_eq!(
            rules(Scope::Document, "Previously, it was slow."),
            ["construct/chronology"]
        );
        assert_eq!(
            rules(Scope::Document, "TODO add a check."),
            ["construct/planned-work"]
        );
        assert!(rules(Scope::Document, "We adjust the justification.").is_empty());
    }

    #[test]
    fn comments_keep_code_words() {
        assert!(rules(Scope::Comment, "It just returns the cached value.").is_empty());
        assert_eq!(
            rules(Scope::Comment, "It is very slow."),
            ["construct/intensifier"]
        );
    }
}
