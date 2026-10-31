# ADR 012: The weld — equal address sets across a unit's texts · `rec:adr:012-the-weld`

**Decision (Statement)** · `dec:weld:equal-address-sets`

Within each statement unit, the gate checks: every language the document's declaration requires is present; every tag and label addressed resolves; and across the unit's texts, the addressed tag set and the cited label set are equal — compared as sets, mint and cite separately, position never mattering. In authored texts a bare span cites and a doubled span mentions without harvesting; mints are unit metadata and never occur in texts. Failures name the unit and the language.

**Reasons (Why)** · `arg:weld:reasons`

- The set is the narrow waist: each language's text stays free in everything that makes translation worth having — idiom, emphasis, sentence shape — and constrained in exactly one thing, that it addresses the unit's surface. A weld on word positions would make a translation a transliteration; no weld at all would make it a caption.
- Mint and cite compared separately because they assert different things: a mint is identity, a cite is reference, and a translation that cites what the original never cites is wrong in a way a combined set would hide.
- Set semantics make the gate cheap and its failures exact — a unit name and a language, not a document-length diff.

**Declined (Alternatives)** · `arg:weld:declined`

Positional alignment (transliteration by gate); subset tolerance (a translation allowed to drop addresses translates less than the text says); prose-level similarity checks (unreviewable and gameable where sets are neither).

**Consequences (Accepted)** · `arg:weld:consequences`

Authors address the surface identically in every language of a unit, even where a language's idiom would prefer a paraphrase around the handle; that constraint is the price of a translation that provably covers the original's surface.
