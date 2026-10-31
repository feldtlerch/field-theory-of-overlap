# ADR 009: Process — signed reviewed bites, each layer with its validator · `rec:adr:009-process`

**Decision (Statement)** · `dec:process:signed-bites`

Every commit this repository retains is signed by the project's signing key and lands green through continuous integration; there is no mega-import of a prepared tree, and the bootstrap's order is the argument's order — records first, then the workspace skeleton with its checks green while empty, then crates in reviewed bites, each layer landing together with the validator or consumer that checks it, never ahead of it. Commit messages state what holds and why, timelessly; checks report their wall time.

**Reasons (Why)** · `arg:process:reasons`

- A history built in reviewed bites *is* the review record: the sequence of green, signed commits is evidence a prepared tree cannot fake, and a signature on every retained commit makes authorship a verifiable fact rather than a convention.
- The validator-with-layer rule has the strongest empirical record the earlier implementation left: every consumer — the validator over the graph data, the audit over the projection, the byte gates over generated artifacts, the sweep over generated commands — found real defects in work that had passed review without it. A layer merged before its consumer exists is unreviewed in the only sense that matters.
- Checks green while empty, from the first commit, because a check added later inherits a backlog of exceptions; a check present from the start never learns to tolerate red.

**Declined (Alternatives)** · `arg:process:declined`

Importing the design as one prepared tree (review at that scale is skimming with ceremony); unsigned work commits with signed releases (the history between releases then proves nothing); validators as a hardening phase after the build (the earlier record shows exactly what that misses).

**Consequences (Accepted)** · `arg:process:consequences`

Progress is paced by review and by check walls, deliberately; a bite too large to review honestly is split rather than waved through — the process's one non-negotiable friction.
