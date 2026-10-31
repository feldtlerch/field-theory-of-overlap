# ADR 010: Validation rules form a registered, stated-scope rule set · `rec:adr:010-validation-rules`

**Decision (Statement)** · `dec:graph:validation-rules`

The authority's validator is a numbered register of rules, each carrying its state — enforced, structural, or deferred with the reason — and each stating its scope conditions as part of the rule: which node kinds it reads, which anchorings it fires on. Three scope decisions are themselves frozen here: the shared-weighting rule compares same-kind paths only; the reads-through rule fires only for readings anchored in the field; and a converter is checked against its own declared input, conversion and output, never against what derivation edges imply.

**Reasons (Why)** · `arg:graph:validation-rules-reasons`

- A rule that cannot yet be enforced must be visible as a debt rather than silently absent; the register with states is what a reader audits the validator against.
- The sharpest validator defects on record were rules firing outside their meaning: a weighting comparison across kinds produced false conflicts because a holding reads one end's weighting directly while its covector reads the other's; a field-anchoring rule applied to latent readings wrongly caught quantities that never touch the field; and inferring a converter's contract from derivation edges makes the check circular with what it audits.
- Scope stated in the rule is scope reviewable with the rule — the alternative is scope encoded in code paths nobody can cite.

**Declined (Alternatives)** · `arg:graph:validation-rules-declined`

An undifferentiated validation pass (no debt visibility, no per-rule scope); warnings for unenforceable rules (a warning is a rule the build has learned to ignore).

**Consequences (Accepted)** · `arg:graph:validation-rules-consequences`

Every new rule lands with its state and scope and is numbered forever; relaxing or tightening a rule's scope is a reviewed change to the register, not a code tweak.
