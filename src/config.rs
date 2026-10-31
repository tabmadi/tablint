//! `tablint.toml`. Every key has a default, and the defaults are the opinion: an empty file runs every rule.

use std::path::Path;

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;

pub const FILE_NAME: &str = "tablint.toml";

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    /// Paths that no rule reads, as globs.
    pub exclude: Vec<String>,
    /// Rule IDs or rule groups, such as `adr` or `prose/question`, that do not run.
    pub disable: Vec<String>,
    pub prose: Prose,
    pub comments: Comments,
    pub adr: Adr,
    pub docs: Docs,
    pub messages: Messages,
    pub annotations: Annotations,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Prose {
    /// Text adopted from a third party. No prose rule and no character rule reads it.
    pub verbatim: Vec<String>,
    pub max_words: usize,
    pub procedure_max_words: usize,
    /// Documents whose every sentence is a procedure step.
    pub procedure_paths: Vec<String>,
    /// Documents that name the banned constructs, such as the ADR that bans them.
    pub constructs_exempt: Vec<String>,
    /// Documents whose subject is live state, so `currently` is the fact.
    pub status_exempt: Vec<String>,
    /// Technical names with a slash, such as `CI/CD`.
    pub slash_terms: Vec<String>,
    /// Regular expressions for spans that are not prose, such as an evidence grade.
    pub ignore: Vec<String>,
}

impl Default for Prose {
    fn default() -> Self {
        Self {
            verbatim: strings(&[
                "LICENSE",
                "**/CHANGELOG.md",
                "CODE_OF_CONDUCT.md",
                "**/*.local.md",
            ]),
            max_words: 25,
            procedure_max_words: 20,
            procedure_paths: strings(&["docs/guide/**"]),
            constructs_exempt: strings(&["docs/adr/0001-*.md", "docs/adr/_template.md"]),
            status_exempt: Vec::new(),
            slash_terms: strings(&["ci/cd", "i/o", "tcp/ip"]),
            ignore: strings(&[
                r"\((?:reasoned|documented|measured)\b[^()]*(?:\([^()]*\)[^()]*)*\)",
            ]),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Comments {
    /// The files whose comments the rules read, as globs. Empty means every file.
    pub paths: Vec<String>,
    pub max_lines: usize,
    /// Directory names that hold generated or vendored code.
    pub skip_dirs: Vec<String>,
    /// File name suffixes of generated code.
    pub skip_suffixes: Vec<String>,
    /// Comment prefixes that a tool in the repository reads, beside the built-in list.
    pub directives: Vec<String>,
    /// Pairs of markers. The lines between them are data a generator writes, not comments.
    pub skip_regions: Vec<[String; 2]>,
}

impl Default for Comments {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            max_lines: 3,
            skip_dirs: strings(&[
                "node_modules",
                "vendor",
                "dist",
                "build",
                "target",
                "testdata",
                "_generated",
            ]),
            skip_suffixes: strings(&["_gen.go", ".sql.go", ".pb.go", ".min.js"]),
            directives: Vec::new(),
            skip_regions: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Adr {
    pub dir: String,
    pub index: String,
    pub header_fields: Vec<String>,
    /// The index column that repeats each Decides line must match it word for word.
    pub index_matches_decides: bool,
    /// The section order. A section may be absent, but the ones present keep this order.
    pub sections: Vec<String>,
    /// ADR numbers outside the section rules, such as a foundations ADR with no options to compare.
    pub sections_exempt: Vec<String>,
    /// Words before `ADR-NNNN` that name another repository's set, such as `games`.
    pub foreign: Vec<String>,
    /// Documents where every `ADR-NNNN` must resolve.
    pub reader_paths: Vec<String>,
}

impl Default for Adr {
    fn default() -> Self {
        Self {
            dir: "docs/adr".into(),
            index: "docs/adr/README.md".into(),
            header_fields: strings(&["status", "decides"]),
            index_matches_decides: true,
            sections: strings(&[
                "Context",
                "Decision drivers",
                "Considered options",
                "Decision",
                "Consequences",
                "Rules",
            ]),
            sections_exempt: strings(&["0000"]),
            foreign: Vec::new(),
            reader_paths: strings(&["docs/**", "README.md", "AGENTS.md"]),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Docs {
    /// The genre directories under `docs/`. Empty means any directory.
    pub genre_dirs: Vec<String>,
    /// The file names allowed at the `docs/` root: the entry documents and the registries. Empty means any.
    pub root_files: Vec<String>,
    /// File names that repeat by design, such as the index of each directory.
    pub shared_names: Vec<String>,
    /// Link targets that no document under `from` may point to.
    pub forbidden_links: Vec<ForbiddenLink>,
}

impl Default for Docs {
    fn default() -> Self {
        Self {
            genre_dirs: Vec::new(),
            root_files: Vec::new(),
            shared_names: strings(&["README.md", "_template.md"]),
            forbidden_links: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ForbiddenLink {
    pub from: String,
    pub to: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Messages {
    /// The directory of UI string catalogues, one `<locale>.json` per locale.
    pub dir: Option<String>,
    /// The locale that gets the word rules. Every locale gets the punctuation and length rules.
    pub source_locale: String,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            dir: None,
            source_locale: "en".into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Annotations {
    /// Annotation kinds whose values are `mise` task names, such as `CI` in `(CI: lint)`.
    pub task_kinds: Vec<String>,
}

impl Default for Annotations {
    fn default() -> Self {
        Self {
            task_kinds: strings(&["CI", "check"]),
        }
    }
}

impl Config {
    /// Reads `tablint.toml` from the root, or returns the defaults when the file is absent.
    pub fn load(root: &Path, explicit: Option<&Path>) -> Result<Self> {
        let path = explicit.map_or_else(|| root.join(FILE_NAME), Path::to_path_buf);
        if explicit.is_none() && !path.exists() {
            return Ok(Self::default());
        }
        let text =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
    }

    /// A rule runs unless its ID, or a group that contains it, is in `disable`.
    pub fn enabled(&self, rule: &str) -> bool {
        !self.disable.iter().any(|d| {
            rule == d
                || rule
                    .strip_prefix(d.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
        })
    }
}

pub fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

/// A set of path globs. A `*` does not cross a `/`, and `**` does.
pub struct Paths(GlobSet);

impl Paths {
    pub fn new(globs: &[String]) -> Result<Self> {
        let mut set = GlobSetBuilder::new();
        for g in globs {
            set.add(Glob::new(g).with_context(|| format!("glob {g}"))?);
        }
        Ok(Self(set.build()?))
    }

    pub fn matches(&self, path: &str) -> bool {
        self.0.is_match(path)
    }
}
