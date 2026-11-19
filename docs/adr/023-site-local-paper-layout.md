# ADR 023: Site-local paper layout with two retained coordinates · `rec:adr:023-site-local-paper-layout`

**Decision (Statement)** · `dec:paper:site-local-layout`

Local layout instructions attach to their paragraph, display, heading or box ending, preserve words, formal expressions and reference spellings, and restore parameters at their existing local boundary. The appendix symbol paragraph and the explainer checks formal note retain two line-addressed repairs while their accepted passage bytes remain unchanged; every retained line address has a matching whole-file pin. Moving either instruction into its accepted passage requires separate renewal of that passage's acceptance.

**Reasons (Why)** · `arg:paper:site-layout-reasons`

A local instruction moves with its subject. Exact source-byte acceptance remains meaningful for the two unchanged passages, whose source bytes would change if an instruction were inserted inside them.

**Declined (Alternatives)** · `arg:paper:site-layout-declined`

Relocating inside accepted bytes without renewal loses acceptance. Dropping all repairs confounds coordinate removal with appearance. A new positional register recreates the same dependency.

**Consequences (Accepted)** · `arg:paper:site-layout-consequences`

This supersedes ADR 014's statement “The reader hooks enumerate the paragraphs that need extra line-breaking stretch and restore the prior setting afterward; paragraphs ending in a display receive the same setting inside that display's group.” Source-local instructions select relocated paragraphs; only the two retained accepted-passage sites are selected by source coordinates. The prior settings are restored, and display-entry assignments remain local to the display group.

This also supersedes ADR 014's statement “The remaining page repairs are line-addressed and pinned in the same way, each confined to its site and restored afterward.” Page repairs are site-local except for the two explicitly retained line-addressed sites; every retained coordinate has a matching pin. The existing pin checks, identity and bibliography requirements, and the original look as the page-review baseline continue to apply.
