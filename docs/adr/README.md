# Architecture Decision Records

Each ADR ends with a flat **Rules** section and has a one-sentence **Decides** line in its header. The *Decides* column below repeats that line, so it is the fastest complete pass over the set.

| ADR | Decides |
| --- | --- |
| [0000](0000-foundations.md) | tablint is one owner's linter for prose, comments, and decision records, it is public and free to use, and every decision is an ADR |
| [0001](0001-documentation-conventions.md) | The house style for every document, comment, and UI string: Simple English on the ASD-STE100 model |
| [0100](0100-toolchain.md) | The pinned toolchain, the task surface, what runs at commit and push time, and how the pins stay current |
| [0200](0200-architecture.md) | tablint is one Rust binary that parses a repository once into a model and runs built-in rules with stable IDs over it |
| [0300](0300-releases.md) | Every version is 0.y.z with a plain tag, any release may break a repository, and a release is one archive per target that mise installs |

A new ADR starts from [`_template.md`](_template.md).
