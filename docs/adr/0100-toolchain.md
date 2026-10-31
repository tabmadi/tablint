# ADR-0100: Toolchain

**Status:** Accepted
**Decides:** The pinned toolchain, the task surface, what runs at commit and push time, and how the pins stay current.

## Context

Three failures matter. The first is a tool that resolves from `$PATH`, at the version the machine has. The second is a check that only CI runs, which every local commit passes. The third is a pin that nobody updates.

## Decision

`mise` pins every tool and is the only command surface. A command is `mise run <task>`. A bare tool call resolves from `$PATH` at an unpinned version.

| Tool | Owns |
| --- | --- |
| mise | Tool versions and tasks |
| Rust, with cargo, rustfmt, and clippy | The build, the format, the Rust lint, and the tests |
| lefthook | Git hooks |
| cocogitto | Commit message shape |
| rumdl | Markdown style |
| gitleaks | Secret scanning |
| Renovate | Updates to the pins and to the crates |

| Task | Does |
| --- | --- |
| `setup` | Installs the git hooks |
| `format` | Applies every formatter |
| `lint` | Runs every linter: `lint:rust`, `lint:md`, `lint:secrets`, and `lint:tablint` |
| `test` | Runs the unit tests and the fixture tests |
| `check` | Runs `lint` and `test`. It is the gate before a change is finished |
| `ci` | Runs exactly what CI runs |
| `release:build`, `release:upload` | Build and attach the release archives, per [ADR-0300](0300-releases.md) |

**tablint checks this repo.** `lint:tablint` runs the binary that the working tree builds, so a change to a rule meets this repo's own text first.

A hook has no active mise shell, so each hook command runs its tool through `mise x` or a `mise` task. The `pre-commit` hook runs the fast checks on the staged files. The `pre-push` hook runs `cog check` on the commits that the push adds, and `mise run ci`. Commit messages follow Conventional Commits, and `cog` verifies them in the `commit-msg` hook.

The CI workflow runs `mise run ci` on every push to `master` and on every pull request. On a pull request, it also verifies the title with `cog`, because the title becomes the squash commit. The title reaches the shell only through an environment variable.

Renovate opens a pull request when a pin in `.mise.toml`, a crate in `Cargo.toml`, or a GitHub Action has a new version. The CI gate decides, and Renovate merges nothing.

## Consequences

A push runs the full gate on the local machine first, so a slow gate makes every push slow. `git push --no-verify` skips it, and CI still runs it.

Renovate needs the Renovate GitHub app on the repository. Without it, the pins go stale.

## Rules

- Every tool is pinned in `.mise.toml`. No tool is resolved from `$PATH`.
- Every command is a `mise` task with a `description`. A CI workflow step calls a task, never a tool directly.
- A git hook runs every tool through `mise x` or a `mise` task.
- Commit messages and pull request titles follow Conventional Commits with the type set in `cog.toml`. `(ref: Conventional Commits 1.0.0)`
- `cargo clippy` runs with every warning as an error, and the pedantic group on. `(CI: lint:rust)`
- `mise run check` passes before a change is finished.
- `mise run ci` is the only CI step after setup. `(CI: check)`
- A workflow never puts event text, such as a pull request title, directly into a `run` script.
