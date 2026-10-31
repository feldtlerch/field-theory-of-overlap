# ADR 004: The closed notation AST, two renderers, derived tiers · `rec:adr:004-notation-ast`

**Decision (Statement)** · `dec:notation:closed-ast`

Notation is generated from a closed abstract syntax: primitives with declared arity from a registry, literals that reject structural characters, calls matched to declared signatures. The AST renders to two targets — macro calls for the LaTeX documents, full recursive expansion to Unicode KaTeX for the Markdown documents, with preferred Unicode spellings pinned in the primitive registry. Symbol tiers (authorised against pending) are derived from the graph, never curated, with an alias taking the lesser tier of its own and its target's; an object has one canonical symbol; notation constructors are not quantity commands, and dimensions render with plain brackets. A machine-generated sweep typesets every generated command, and every expanded expression is checked against the KaTeX grammar.

**Reasons (Why)** · `arg:notation:reasons`

- Macros assert nothing: a check written in a typesetting language can be neither reviewed nor reused, and runs too late — the earlier implementation deleted its typed-macro checking layer on exactly this evidence, rather than porting it.
- The closed AST is what makes the second renderer possible: full expansion is mechanical only because every body is data, so the two forms derive from one authored expression and cannot disagree.
- Token boundaries belong to the renderer, decided once for every symbol: a primitive beside a literal once silently concatenated into an undefined command that was declared cleanly and never typeset — hence also the sweep, because command declaration stores bodies unexpanded and proves nothing.
- Every hole in curated tiers (aliases, quantities without a backing object) filed something authorised that was not; derivation closed those holes, including the alias chain, and keeps them closed. A quantity without a backing object stays visibly pending, naming what it awaits.

**Declined (Alternatives)** · `arg:notation:declined`

Arbitrary control-sequence strings in the registry (the handwritten layer's freedom, returned one string at a time); a curated authorised list (drifts); per-symbol markup authored for the Markdown target (a second authored form for one meaning — the drift class this repository exists to kill).

**Consequences (Accepted)** · `arg:notation:consequences`

Admitting a primitive is a reviewed registry change checked against what already composes; the paper's full symbol vocabulary enters only as fast as the referent expansion carries it.
