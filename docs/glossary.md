# Glossary · `reg:glossary:terms`

- **Authority** — the one authoritative representation of the paper's semantic content: a typed graph of theory objects and their relations, held as Rust data (`rec:adr:001-closed-typed-graph`).
- **Claim** — a theory statement the authority registers and the audit can certify, refute, or report undecided or deferred.
- **Coverage** — the declared boundary of the audited slice: which claims the audit currently reaches, with a release gate distinguishing a complete release from an explicit partial one.
- **Label** — a `kind:area:slug` identifier minted once at the statement or object it names; the machine-facing identity layer.
- **Tag** — a short human-facing handle bound to a label; the form prose uses to address a theory object (`rec:adr:003-tags-bound-to-labels`).
- **Unit** — a statement unit: the authoring element of prose, carrying metadata, shared mathematics and one text per language, from which the documents are generated (`rec:adr:005-statement-units`).
- **Weld** — the gate holding a unit's texts to one addressable surface: the tag and label sets each text addresses must be equal (`rec:adr:012-the-weld`).
- **Producer** — the audit function registered for one claim, constructing the claim's mathematical statement for the calculation engine (`rec:adr:007-audit-on-the-engine`).
- **Referent** — a theory object admitted to the authority; the **referent expansion** is the ongoing admission of objects the prose already uses, in the order the claims under audit need them.
- **Bite** — one reviewed, gated, signed change; the unit of this repository's history (`rec:adr:009-process`).
- **Slice** — the audited part of the theory at a given moment, as the coverage declaration states it.
- **Display** — a figure or table: a unit whose bodies are authored per rendering target rather than shared across them.
- **Skeleton** — a document's ordered tree of sections, structure only, naming the units each section contains.
- **Explainer** — a companion publication writing the paper's ideas for a reader who will not read the proofs; authored as units like the paper, with its own declared language set.
- **Projection** — the generated artifact that carries the authority into the toolchain: the one data file the typesetting runtime and the audit both consume.
- **Pin** — the recorded public commit of the calculation engine the audit depends on; moved only along that repository's main branch, by review.
- **Verdict** — the audit's typed outcome for one claim: certified, refuted, undecided with what ran out, or deferred with the named capability.
- **Ruling** — a registered editorial judgment of the theory (a classification the authority records as an object of its own, with status like any other).
