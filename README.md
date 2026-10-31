# tablint

An opinionated linter for Simple English prose, code comments, and decision records. It checks the house style of [ADR-0001](docs/adr/0001-documentation-conventions.md) and the structure of an ADR set, in one binary.

tablint serves one owner's repositories. It is free to use under the Apache License 2.0, but it makes no promise of support or compatibility. Every version is 0.y.z, and any release may break your gate.

## What it checks

| Group | Checks |
| --- | --- |
| `simple` | Punctuation, contractions, Latin abbreviations, complex words, emoji, and sentence length in prose, comments, and UI strings |
| `construct` | Intensifiers, hedges, meta-commentary, chronology, status words, planned work, and dated headings |
| `char` | The em dash, en dash, ellipsis, and curly quotes, in every text file |
| `comment` | Block length, paragraphs, commented-out code, author lines, banners, uncited `TODO`s, and doc comments that restate the name |
| `adr` | The header, the one-sentence Decides line, the index, references, Related, and the section order |
| `docs` | Genre directories, the docs root, repeated filenames, single-file directories, and links to untracked files |
| `annotation` | A `(CI: <task>)` annotation names a `mise` task that exists |

`tablint rules` lists every rule ID. Comments are read in Go, TypeScript, JavaScript, Rust, Python, GDScript, shell, TOML, YAML, and SQL.

## Getting started

1. Pin tablint in `.mise.toml`, at an exact version:

   ```toml
   [tools]
   "github:tabmadi/tablint" = "0.1.0"
   ```

2. Add a task:

   ```toml
   [tasks."lint:tablint"]
   description = "Prose, comments, and ADRs, per ADR-0001"
   run = "tablint check"
   ```

3. Run it with `mise run lint:tablint`. Each finding is one line: `path:line: rule: message`.

## Configuration

An empty `tablint.toml` runs every rule with its defaults. A key sets a path or turns a rule off:

```toml
exclude = ["vendor/**"]
disable = ["construct/status", "docs"]

[adr]
foreign = ["games"]
index-matches-decides = false

[docs]
genre-dirs = ["adr", "guide", "reference"]
root-files = ["README.md", "tool-register.md"]

[messages]
dir = "apps/frontend/src/messages"
```

A line opts out of every rule with `tablint:allow`, in an HTML comment in Markdown or in a comment in code.

## Decisions

The ADRs in [docs/adr](docs/adr/README.md) record every decision and the reason for it.

## License

Apache License 2.0. See [LICENSE](LICENSE).
