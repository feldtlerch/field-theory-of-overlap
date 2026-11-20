# ADR 025: Identified progressive reading trees · `rec:adr:025-identified-reading-trees`

**Decision (Statement)** · `dec:reading:identified-spines`

Each reading is a document's ordered structure through shared content. The paper and explainer are the two readings. Each identified structure is progressively populated: before a reading has a checked root, its code-declared occurrence identities are allocations and its checked skeleton remains its order and ownership authority. Once a root is constructed, its placed nodes form one containment tree whose ordered children determine reading order; allocations remain outside that tree until placed in their actual enclosing scopes. Until a document's complete transfer, its checked skeleton supplies native unit order, membership and typed quotation; transfer replaces that authority atomically with the complete owning reading inventory. Reading identities precede theory-content admission; a presenting link exists only when its theory target exists. Structural variation stays inside the one reading, under its one root, as branches on separately identified structural decisions; realization, including language, is discharge and never structure.

**Rule (closed identities)** · `rule:reading:closed-identities`

Reading identities are plain hand-written closed enumerations with stable semantic names. One exhaustive specification supplies current fixed facts and kind; ordinary code constructs allocations and their checked catalogue, while placements and projections enter with their first production consumers. Private identity enums are exposed only through checked opaque keys. Kinds and bodies enter with their first production use. Semantic names satisfy the label grammar with kind read. Ordinary admission never reuses a retired name; a changed referent requires ADR 026's historical-resolution warrant. These declarations are the only reading identity authority.

**Rule (rooted order)** · `rule:reading:rooted-order`

A placed reading has one root, acyclic ordered containment and exactly one parent per non-root node.

**Rule (aligned refinement)** · `rule:reading:aligned-refinement`

Pure refinement identifies a node's own logical children top-down at their boundaries, preserving existing identity, public targets, parent and sibling order. A deliberate kind change, regrouping, merge or move is a structural change with its own reason and revalidation, not concealed refinement.

**Rule (explicit structural change)** · `rule:reading:explicit-structural-change`

A structural change records its old/new interpretation and rechecks allowed children, body coverage, relations, destinations, numbering and realization; conceptual warrant follows ADR 026.

**Rule (body accounting)** · `rule:reading:body-accounting`

Every body is emitted once or replaced by a complete disjoint ordered partition; unresolved interiors and events are explicit, and event children annotate without duplicating text.

**Rule (ownership and quotation)** · `rule:reading:ownership-and-quotation`

Until complete transfer, checked skeleton membership supplies ownership; afterward each native unit has one owning document and an owning reading occurrence, including root metadata. Quotations identify another reading's owning occurrence and add no ownership or mint; writers that emit them share its language text and destination and give an explicit unlinked notice for an unavailable owner.

**Rule (reserved names)** · `rule:reading:reserved-names`

The semantic label kinds read and audit belong respectively to reading declarations and derived audit identities, not to theory/documentary mints or their tag targets; handle spellings remain uninterpreted.

**Reasons (Why)** · `arg:reading:identified-spines-reasons`

Exhaustive compiler-checked declarations keep identity authority in the same form as the theory graph without a generator or bootstrap dependency. Ordinary construction avoids a second roster or order. Allocation gives existing appearances stable names before their enclosing outline is known, without inventing parents that later have to move. Conditional transfer preserves one active order and text owner throughout conversion.

**Declined (Alternatives)** · `arg:reading:identified-spines-declined`

A data identity registry moves closed-identity mistakes to runtime admission. Generated enums add an ordering and reproduction boundary. Separate successor and child lists duplicate order. An empty production catalogue provides no useful inventory evidence. Separate documents for structural variations duplicate authority; one compulsory sequence suppresses adaptation. Language-keyed alternatives confuse text selection with decisions. Bottom-up partitions cross logical scopes; replacing the skeleton before its complete transfer leaves ambiguous ownership.

**Consequences (Accepted)** · `arg:reading:identified-spines-consequences`

Names, payload arms and construction calls cost about three edit sites per new occurrence, with body boundaries additional where needed. No ALL list, macro identity table or generated source is required. A coarse tree does not certify hidden TeX events or complete native screens. Legacy bodies remain explicit until replacement, and unavailable screen content remains honestly accounted for.

Extend ADR 001:5's “node identifiers form a closed enumeration serialized by stable semantic name” to reading identifiers. Its closed kind/relation sets remain; ADR 026 governs warranted changes without weakening current validation. Scope ADR 002:5's “Every object carries a four-axis status — maturity, identification, calibration, evidence — with values derived wherever derivable” to theory content; M074 checks constructed allocations, while reading lifecycle, body state and availability gain separate structural checks with their first admitted consumers and confer no mathematical status.

Replace ADR 005:5's “Documents declare their required language sets and assemble through skeletons: ordered trees of sections headed by units, structure only, each unit in exactly one skeleton.” with “Documents declare their required language sets; until a document's complete transfer its checked skeleton, an ordered tree of sections headed by units and carrying structure only, supplies order and single unit membership; afterward its identified reading tree supplies resolved order and one owning occurrence per unit, including metadata attached to the root as a typed unit; headings may temporarily resolve checked legacy source.” Per-language text and per-target displays retain their existing contracts.

Replace ADR 016:13's “a unit that belongs to exactly one skeleton may be quoted in the other document by a typed quotation-use without a second mint or authored copy” with “a unit with one checked owner may be quoted in another document without a second mint or authored copy; until complete transfer the checked skeleton supplies that owner, and afterward its owning reading occurrence does so”. Replace “A quotation-use names the quoted unit, renders its shared text and links to its destination in the document whose skeleton contains it; it places the unit in no second skeleton, so each unit stays in exactly one skeleton” with “Until a document's complete transfer, a quotation-use names its shared unit and checked skeleton destination; afterward a quotation occurrence names its owning reading occurrence and shares the text and destination of the unit owned by that occurrence; neither form adds membership”. Replace ADR 016:37's “Document assembly and emission must check single skeleton membership and typed quotation” with “Document assembly and emission must check single skeleton membership and typed quotation until a document's complete transfer; after transfer, assembly and emission must check one owning document per native unit, unique owning occurrences and typed quotation occurrences”. Retain its separate paper-preparation clause as qualified by ADR 024 and the rest of that paragraph. Each transfer's existing machine rules change when the consumer transfers, not when this record is accepted.
