# ADR 005: Statement units — one text per language, per-target displays · `rec:adr:005-statement-units`

**Decision (Statement)** · `dec:units:statement-units`

Prose is authored as statement units in TOML files: a unit's id is its minted label (metadata, never a span in text), mathematics is written once per unit in TeX math using the generated notation, and texts are keyed by language — one authored text per language, from which the LaTeX and Markdown documents are both generated. Displays (figures, tables) instead carry one authored body per rendering target, side by side in the unit. Documents declare their required language sets and assemble through skeletons: ordered trees of sections headed by units, structure only, each unit in exactly one skeleton. The main document declares English alone; translations live in the explainer documents, each language entering its explainer's declaration as it is brought current.

**Reasons (Why)** · `arg:units:reasons`

- Co-location dissolves drift where it starts: the earlier implementation maintained a hand-written translation beside the paper and welded them after the fact, and every revision paid the gate's price for a separation the authoring form itself had created.
- One text per language, because a LaTeX source and a Markdown mirror of the same words only ever differ in markup, and markup maintained by hand is the toolchain's job; per-target display bodies, because a display's best form is target-idiomatic — a diagram that is excellent TikZ makes poor Markdown, and a flattened substitute in both serves neither reader.
- A per-unit gate fails by unit id and language — the difference between a diff and an afternoon — and a declared language structurally cannot lag silently, which turns a translation-staleness class of defect from a standing debt into an impossibility.
- Translation effort concentrates where the audience is broad: an explainer exists for the reader the paper's register excludes, so a translated explainer serves more readers than a translated theorem; the declaration mechanism makes this costless to revisit.

**Declined (Alternatives)** · `arg:units:declined`

Authored LaTeX beside authored Markdown per language (the old pair, one level down); a translation generated from the authority plus fragments (kills the translation as a text anyone wrote); prose-minted labels (a mint is the unit's identity, and texts that mint invite one statement minted twice).

**Consequences (Accepted)** · `arg:units:consequences`

The existing paper migrates into units part by part — a real conversion cost paid alongside the tag wiring; the authoring markup is deliberately small and grows only by reviewed extension; and semantic equivalence across a display's target forms is the review's named judgment, since no gate can check that a picture says what a table says.
