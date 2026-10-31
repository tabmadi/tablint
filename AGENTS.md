# Agent guide

This guide is for any coding agent that works in this repo. `AGENTS.md` is the one standard, and the repo has no per-tool files such as `CLAUDE.md`.

**Humans are the first developers. The ADRs, `docs/`, and `README.md` rank above this file.** This file holds only operational hints.

## Read first

- [ADR-0000](docs/adr/0000-foundations.md): what tablint is and the ADR process.
- [ADR-0200](docs/adr/0200-architecture.md): the repository model, the rule IDs, and the fixtures.
- [docs/adr/README.md](docs/adr/README.md): the ADR index. To check a convention, grep the `Rules` sections first.

## Write in Simple English

Every text follows the Simple English profile in [ADR-0001](docs/adr/0001-documentation-conventions.md#simple-english): docs, ADRs, comments, commit titles, and task descriptions. `mise run lint:tablint` checks it with the binary this repo builds.

- Use common words. One idea per sentence, at most 25 words.
- In prose, use only period, comma, colon, and the possessive apostrophe. No em dash, semicolon, parentheses, or `?`.
- No emoji, no contractions, and no `e.g.` or `etc.`
- No chronology, intensifiers, hedges, or planned work. State what is true now.

## Where things are

| Path | Holds |
| --- | --- |
| `src/repo.rs`, `src/markdown.rs`, `src/comments.rs` | The repository model: files, Markdown views, and comment blocks |
| `src/simple.rs`, `src/constructs.rs` | The word lists and patterns of ADR-0001 |
| `src/rules/` | One module per rule group, and `RULES`, the table of every rule ID |
| `src/config.rs` | `tablint.toml` and its defaults |
| `tests/fixtures/` | Small repositories that break rules on purpose, with their exact output |

## Working in the repo

- Run every command as `mise run <task>`. A bare tool call resolves from `PATH` at an unpinned version.
- A new rule gets an ID in `RULES`, a case in a fixture, and a line in the README table.
- After a deliberate change to the output, run `TABLINT_BLESS=1 mise run test` and review the diff of the `.expected` files.
- Before you finish a change, run `mise run check`.
- Commit messages are a Conventional Commits title only, with no body. Never push unless asked to.
