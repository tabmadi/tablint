# ADR-0300: Releases

**Status:** Accepted
**Decides:** Every version is 0.y.z with a plain tag, any release may break a repository, and a release is one archive per target that mise installs.

## Context

tablint has one owner and no promise of compatibility, per [ADR-0000](0000-foundations.md). A version number that claims stability would be false.

Installers such as the `mise` GitHub backend skip a pre-release when they resolve a version. A version marked as a beta would be hard to install.

## Decision

**The major version is 0, always.** Under Semantic Versioning, a 0.y.z release may change anything. A new or stricter rule bumps the minor version, and a fix bumps the patch version.

**The tag is plain.** A release is `v0.y.z`, with no suffix such as `-beta`. The GitHub release is not marked as a pre-release.

**A repository pins an exact version** in its `.mise.toml`, as `"github:tabmadi/tablint" = "0.y.z"`. A new version reaches it only through a pin bump, and its gate decides.

**A tag builds the release.** `.github/workflows/release.yml` builds one archive per target and attaches it to the release. The archive name carries the target triple, so `mise` picks the archive for its platform.

| Target | Archive |
| --- | --- |
| `x86_64-unknown-linux-musl` | `.tar.gz` |
| `aarch64-unknown-linux-musl` | `.tar.gz` |
| `x86_64-apple-darwin` | `.tar.gz` |
| `aarch64-apple-darwin` | `.tar.gz` |
| `x86_64-pc-windows-msvc` | `.zip` |

## Consequences

A release can fail a repository's gate. The repository then keeps its pin until it fixes the findings.

The crate is not published to crates.io. `publish = false` in `Cargo.toml` prevents it.

## Rules

- Every version is `0.y.z`. A tag is `v0.y.z`, with no suffix.
- A GitHub release is never marked as a pre-release.
- A new or stricter rule bumps the minor version.
- Every archive name carries its target triple.
- The crate is never published to crates.io.
