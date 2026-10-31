# ADR-0200: Architecture

**Status:** Accepted
**Decides:** tablint is one Rust binary that parses a repository once into a model and runs built-in rules with stable IDs over it.

## Context

The rules come from [ADR-0001](0001-documentation-conventions.md). They read three kinds of text: Markdown prose, code comments in many languages, and the structure of an ADR set across files.

A trial replaced the owner's Go linters with Vale for the prose and Conftest with Rego for the cross-document rules. It matched every finding. It also added two tools and two rule languages, and the size of the rules did not shrink: the content moved from Go into YAML and Rego. The comment parser stayed custom, because Vale does not read shell or GDScript comments.

## Decision drivers

1. **One language for every rule.** An agent or a person reads and changes a rule in one place.
2. **One parse, many rules.** A rule reads a model of the repository, not raw bytes.
3. **A static binary for each platform.** `mise` installs it with no runtime.

## Considered options

| Option | Driver 1 | Verdict |
| --- | --- | --- |
| **One Rust binary with built-in rules** | met | **Chosen.** The owner chose Rust over Go to gain experience with it *(reasoned)* |
| One Go binary with built-in rules | met | Runner-up. It would reuse the Go linters as they are *(reasoned)* |
| Vale and Conftest, with a Go extractor | unmet: YAML, Tengo, Rego, and Go | Rejected after the trial *(measured)* |

## Decision

**The repository model is built once.** `git ls-files` gives the files: everything tracked, and anything new that git does not ignore. Each Markdown file has two views. The prose view comes from pulldown-cmark: one unit per paragraph, heading, list item, and table cell, with emphasis removed and a code span as one word. The fact view comes from a line scan: header fields, headings, tables, links, annotations, and `ADR-NNNN` mentions. Each source file in a known language becomes comment blocks, with strings, heredocs, and generated regions marked as data.

**A rule is a function with a stable ID.** The ID is `<group>/<name>`, such as `simple/semicolon` or `adr/index`. `tablint rules` lists them. An ID never changes its meaning. A rule with a new meaning takes a new ID.

**The defaults are the opinion.** An empty `tablint.toml` runs every rule. A key sets a path or turns a rule off, per [ADR-0000](0000-foundations.md).

**A line opts out with `tablint:allow`.** In Markdown it is an HTML comment, and in code it is a comment on the same line.

**The output is one finding per line**, as `path:line: rule: message`. The exit code is 0 for no finding, 1 for a finding, and 2 for an error.

**Each rule has a fixture.** `tests/fixtures/<name>/` is a small repository that breaks rules on purpose, and `<name>.expected` holds the exact output. `TABLINT_BLESS=1 cargo test` writes it again after a deliberate change.

## Consequences

A rule that only one repository needs is still code in tablint. When it is too narrow for a built-in rule, the repository keeps its own check.

A regular expression from the Go linters ports to the `regex` crate as it is. Neither supports look-around.

## Rules

- Every rule has an ID of the form `<group>/<name>` and a row in `rules::RULES`.
- A rule reads the repository model, never the file system.
- A rule never duplicates a check of golangci-lint, Biome, rumdl, shellcheck, or gitleaks.
- Every rule has a case in a fixture, and the fixture output is exact. `(CI: test)`
- No crate that needs a C toolchain is added. The release builds stay static.
