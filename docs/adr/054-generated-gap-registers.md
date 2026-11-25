# ADR 054: Generated gap registers · `rec:adr:054-generated-gap-registers`

**Decision (Statement)** · `dec:documents:generated-gap-registers`

Document generation writes `registry/html-block-gaps.json` and `registry/screen-destination-gaps.json` from the checked plan; changes to either register remain reviewable diffs, and ordinary release still refuses while any gap is registered.

**Reasons (Why)** · `arg:documents:generated-gap-registers-reasons`

A derived file is cheaper to generate than to keep by hand, while its diff exposes changed gaps for review and the release refusal preserves the complete-document boundary.

**Declined (Alternatives)** · `arg:documents:generated-gap-registers-declined`

Hand-maintaining the two derived inventories duplicates the plan's ownership of their rows.

**Consequences (Accepted)** · `arg:documents:generated-gap-registers-consequences`

The registered product check still refuses a register that differs from the plan; generation can bring the register to the plan's canonical bytes, and release still refuses a registered gap.

**Amendment (Registered check)** · `amend:documents:generated-gap-registers`

| Earlier clause | Qualification governed by this record |
| --- | --- |
| ADR 046:5: “The product-only `paper check-registered-html-gaps --built` command compares the ordered register with the plan's complete gap inventory, so an added, removed or changed gap refuses; it then requires all release parts other than HTML block gaps to pass and requires the ordinary release refusal at HTML block gaps to name the registered inventory.” ADR 040:29 replaced it with: “The product-only `paper check-registered-html-gaps --built` command compares both ordered registers with their complete plan inventories, so an added, removed or changed gap refuses; it then requires all release parts other than HTML block gaps and screen destinations to pass. The ordinary release regression test `support_required_release_refuses_each_registered_gap_part` requires a refusal at each nonempty part that names its inventory.” | The product-only check refuses a mismatch between either register and the plan; document generation writes the canonical registers, so a changed gap appears as a register diff for review. The remaining release parts and the ordinary release refusals of ADR 040 continue to run. |
