# ADR 008: Publication — fresh history and the severity gate · `rec:adr:008-publication`

**Decision (Statement)** · `dec:publication:fresh-history`

This repository starts with a fresh history, and every crate is re-implemented clean: no code moves from the earlier private implementation, whose history is not published. Anything published passes the publication audit (`rec:policy:publication-audit`): every finding classed, doubt taking the higher class, zero critical and zero major findings before a cut. Tooling dependencies enter only at publicly published pinned versions; until a tool is public, the repository builds without it.

**Reasons (Why)** · `arg:publication:reasons`

- The design converged over several private generations, and the converged design deserves a history that reads as designed once; a public reader deserves a tree with no process residue. The earlier history's value — settled decisions and hard-won failure knowledge — travels as these records instead.
- A copied module imports unreviewed assumptions along with its text; re-implementation is what makes "everything here passed this repository's process" true rather than aspirational.
- The severity scale measures what a reader can do with a publication, not what a line looks like alone — hence doubt upward, and reconstruction counting as major.

**Declined (Alternatives)** · `arg:publication:declined`

Publishing the earlier history after scrubbing (history rewriting at scale is error-prone exactly where errors are critical); vendoring private tooling (publishes private internals by the side door the fresh history closed); per-case severity judgments (a scale that varies by case is not a scale).

**Consequences (Accepted)** · `arg:publication:consequences`

Earlier evidence does not transfer by citation: this repository re-establishes its gates' evidence from its first commit. Some gates wire later than the design wants them, waiting on tooling publications, and the repository says so rather than carrying private copies.
