# ADR 058: Source-bounded paragraph slots · `rec:adr:058-paragraph-source-boundaries`

**Decision (Statement)** · `dec:documents:source-bounded-paragraph-slots`

A declared source interval may transfer to one coarse native unit containing ordered paragraph bodies and checked uses of existing native units, optionally after an admitted heading; its paragraph bodies satisfy M055's source-boundary and unchanged-prefix conditions. The admitted kinds are source paragraph bodies and heading-led prose tails under M055, with decoded text, strong and emphasis runs and named en-space intent, optionally preceded in the explainer by an unnumbered section heading.

**Reasons (Why)** · `arg:documents:source-bounded-paragraph-slots-reasons`

A source block edge is not evidence of a printed paragraph boundary. Captured blank lines and checked heading prefixes provide the narrower boundary evidence needed to preserve an inherited paragraph while moving its prose into native data. The slot retains its own source and document identity for independent comparison.

**Declined (Alternatives)** · `arg:documents:source-bounded-paragraph-slots-declined`

A comment or a display, list, float, label or other command boundary does not by itself establish a paragraph boundary. A heading-led tail does not carry its inherited heading, label or grade into the new unit.

**Consequences (Accepted)** · `arg:documents:source-bounded-paragraph-slots-consequences`

Paragraph-only slots remain excerpts on screens. The inherited heading and checking effects remain outside the selected interval. The full paper pair and unchanged outside envelope remain the retention check.

**Amendment (Bounded corpus composition)** · `amend:documents:source-bounded-paragraph-slots`

The following exact clauses of ADR 055 and rules M055 and M077 are replaced; all other obligations remain.

| Clause | Before | After |
| --- | --- | --- |
| ADR 055:5 first sentence | A declared source interval may transfer to one coarse native unit containing an ordered heading, paragraphs and checked uses of existing native units. | A declared source interval may transfer to one coarse native unit containing ordered paragraph bodies and checked uses of existing native units, optionally after an admitted heading; its paragraph bodies satisfy M055's source-boundary and unchanged-prefix conditions. |
| ADR 055:5 admitted kinds | The initial admission is an unnumbered section heading and paragraphs with decoded text, strong and emphasis runs and named en-space intent. | The admitted kinds are source paragraph bodies and heading-led prose tails under M055, with decoded text, strong and emphasis runs and named en-space intent, optionally preceded in the explainer by an unnumbered section heading. |
| M055 corpus-slot composition | A bounded corpus-slot is a distinct composition mode: one unique source marker is replaced by the canonical multiline body of a checked coarse unit. Its heading form contains an ordered unnumbered section heading, paragraphs with decoded text, strong and emphasis runs and named en-space intent, and unchanged uses of existing native paragraph units. Its result form contains one checked registered insight, remark or definition header, its actual optional grade and admitted grade gap, and ordered independently admitted plain paragraph children. | A bounded corpus-slot is a distinct composition mode: one unique source marker is replaced by the canonical multiline body of a checked coarse unit. Its heading form contains source paragraph bodies with decoded text, strong and emphasis runs and named en-space intent, and unchanged uses of existing native paragraph units; in the explainer an unnumbered section heading of the admitted form may precede them. Each source paragraph body begins after a blank line or, as a heading-led prose tail, after an unchanged section or subsection and its own label and optional section grade; it ends before a blank line or a section or subsection. A run-in heading or printed section grade remains outside a heading-led prose tail as the unchanged prefix of the same TeX paragraph, and the emitted body begins with that tail's prose itself. A paragraph-only slot's replaced interval contains no heading, label, mint, checking effect or layout event; those around it remain outside and unchanged. A comment or a display, list, float, label or other command boundary does not by itself establish a paragraph boundary. Other continuations, nested bodies and paragraphs with layout events require their own admission. Its result form contains one checked registered insight, remark or definition header, its actual optional grade and admitted grade gap, and ordered independently admitted plain paragraph children. |
| M077 corpus-slot correspondence | The corpus-slot route admits one ordered unnumbered section heading, styled paragraphs and unchanged existing native uses as one selected occurrence; every child's correspondence is checked independently on each side within the uniquely matched complete sequence. | The corpus-slot route admits source paragraph bodies and heading-led prose tails under M055, optionally preceded by the admitted unnumbered explainer section heading, and unchanged existing native paragraph uses as one selected occurrence; every child's correspondence is checked independently on each side within the uniquely matched complete sequence. The slot's own paragraph runs are decoded text, strong and emphasis and named en-space intent; any other run refuses the selected occurrence. |
