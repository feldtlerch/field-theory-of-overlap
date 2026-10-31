# ADR 001: The authority is a closed typed graph · `rec:adr:001-closed-typed-graph`

**Decision (Statement)** · `dec:graph:closed-authority`

The paper's semantic authority is a typed graph in Rust: node identifiers form a closed enumeration serialized by stable semantic name; node kinds, relation kinds, guard forms and status values are closed sets; every relation kind declares its permitted endpoint kinds and its acyclicity obligation; spaces are node-indexed families so that one typing rule checks every instance; and guards are a closed syntax with no prose form. Extending any closed set is a reviewed change that states what the new member is: a node kind enters on two members or a demonstrated structural need, and a relation kind must additionally show which wrong edge its constraints refuse.

**Reasons (Why)** · `arg:graph:closed-authority-reasons`

- A compiler-checked identifier cannot be typo'd into a new object; a string can, and serialization by variant name couples the wire format to refactors, which stable semantic names decouple.
- Per-relation declarations are where the validator's power lives: a derivation relation must be acyclic while a compatibility relation may cycle, and that difference cuts across any coarser grouping — the earlier private implementation proved this by needing it.
- Every opening left in earlier schemas admitted both new ideas and defects through the same door; a prose guard in particular is uncheckable by construction and invites the authority to restate the paper instead of modeling it.
- Unindexed spaces forced per-symbol typing; node-indexed space families are what let sums and readings be checked once, for all instances.

**Declined (Alternatives)** · `arg:graph:closed-authority-declined`

Open string identifiers with a linter (the linter sees the data only after the defect is in it); hashes for identity (a hash says "changed", never "wrong"); acyclicity on edge families rather than relation kinds (demonstrably too coarse).

**Consequences (Accepted)** · `arg:graph:closed-authority-consequences`

Admitting a theory object means touching the enumeration — deliberate friction, accepted as the review point the admission process needs. A wrongly modeled object is never edited into a different object: it retires with its reason recorded and a corrected object enters with a new identity, so old texts and retained audit reports keep meaning what they meant.
