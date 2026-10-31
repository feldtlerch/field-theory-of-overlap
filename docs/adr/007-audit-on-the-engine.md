# ADR 007: The audit runs on a public certified-calculation engine · `rec:adr:007-audit-on-the-engine`

**Decision (Statement)** · `dec:audit:engine-of-record`

The audit crate depends on the Naturrhythmuszentrum engine — a public calculation engine with exact CAS structure, certified rational-endpoint enclosures and typed proved/refuted/undecided outcomes, at https://github.com/naturrhythmuszentrum/naturrhythmuszentrum — at a pinned public commit moved only along that repository's `main`; the pin lands with the audit crate's first bite. Producers in a closed registry keyed by claim identity construct each claim's mathematical statement from engine expressions; the registry fails closed in both directions, and the runner's exit codes separate semantic invalidity, mathematical failure and input error. Producers execute in premise order from the graph; a claim supported by exclusive routes is audited on each route alone, never on their conjunction. The boundary with the engine runs on one test: a capability another project could state without naming this theory belongs upstream, requested through that repository's public issues; what names this theory stays in the audit crate's theory module. Budgets for numeric work live in a register in the authority; the runner is one process with a fresh engine store per claim; the report splits into gated verdicts — a pure function of projection, budgets and pin, retained per release — and ungated measurements.

**Reasons (Why)** · `arg:audit:reasons`

- A certified enclosure with a typed undecided outcome is a stronger audit than a float compared to a float, and a public pinned dependency makes the audit reproducible by any reader; the engine's public issue tracker already carries a completed outside contribution in exactly that shape (its first issue, closed with the capability landed under the engine's conventions).
- Closed registry, both directions: an injectable formula makes the audit audit the injection; a claim without a producer and a producer without a claim are input errors before mathematics runs.
- Premise order, because a claim must not be checked before the family it rests on — alphabetical execution once checked a stationary map before the distribution family under it; route exclusivity, because routes that cannot hold together prove nothing jointly.
- A fresh store per claim, because the engine guarantees verdict *validity* is cache-independent but *reaching* one within a budget may not be, and a verdict that flips on another claim's leftover cache is nondeterminism inside the gated part.
- Budgets in a reviewed register because a default chosen by nobody is a verdict chosen by accident; the engine's enclosure costs are measured facts, so each claim's budget is a stated choice.

**Declined (Alternatives)** · `arg:audit:declined`

A computer-algebra audit on floating-point checks (the predecessor: no typed undecided, no public reproducibility); implementing missing mathematics inside the crate (a private CAS growing under a shim — the boundary failing); claim statements as graph data from the start (a second expression language designed before first contact with the engine; revisited per claim as the engine relationship matures).

**Consequences (Accepted)** · `arg:audit:consequences`

The audit's reach is coupled to the engine's public surface: claims wait, typed and visible, on capabilities the public issues will carry — at the outset symbolic trigonometry, formal truncated power series, and integration of trigonometric integrands; the issues are filed with the audit crate's first bite and linked from the README. Re-deriving shared constants per claim costs work the budgets register pays explicitly; moving the pin is a reviewed change that can alter verdicts, which is exactly why the verdicts record it.
