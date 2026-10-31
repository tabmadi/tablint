# ADR-0000: Foundations

**Status:** Accepted
**Decides:** tablint is one owner's linter for prose, comments, and decision records, it is public and free to use, and every decision is an ADR.

## Context

The owner's repositories share one house style and one way to record decisions. Each repository checked them with its own copy of the same linters, in Go, in GDScript, or by review alone. The copies drifted apart, and a fix in one never reached the others.

The established linters each own one language or one format. None of them checks the Simple English profile, the comment rules, and the ADR set together.

## Decision

tablint is one binary that checks the rules of [ADR-0001](0001-documentation-conventions.md) and the structure of an ADR set. A repository installs it with `mise` and adds a short `tablint.toml`.

**It serves its owner.** The rules are the owner's opinion, and a default is a decision, not a starting point. A repository can turn a rule off, but it cannot redesign one. The code is public under the Apache License 2.0, so anyone can use it. It is not advertised, and it makes no promise of support or compatibility.

**It checks only what nothing else checks.** Code style, Markdown structure, shell, and secrets belong to golangci-lint, Biome, rumdl, shellcheck, and gitleaks. tablint never duplicates one of them.

**Every decision is an ADR.** An ADR states what is true of the repo. It is law, not a plan: no roadmap, no follow-ups, and no remark on unfinished work. A gap between an ADR and the code is unfinished work.

**Planned work stays local.** It lives in an untracked `*.local.md` file, and `.gitignore` excludes these files.

## Consequences

A rule that suits the owner and not another user stays. Another user forks the repository or turns the rule off.

One fix to a rule reaches every repository at its next version bump, through its `mise` pin.

## Rules

- Every ADR carries a `Status`, a one-sentence `Decides` line, and a flat `Rules` section. `(CI: lint:tablint)`
- An ADR states what is true, never what is intended. No roadmap, follow-up, or status section.
- A rule that another tool in the toolchain of ADR-0100 already checks is not added to tablint.
- A repository changes a rule only by turning it off or by setting a path in `tablint.toml`.
- Planned work lives in an untracked `*.local.md` file. No committed file links to one. `(CI: lint:tablint)`
