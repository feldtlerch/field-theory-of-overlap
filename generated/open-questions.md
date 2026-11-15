<!-- generated open-question register; edits are refused by the byte gate -->
# Open questions

Generated from `registry/open-questions.toml`; it lists the records the store holds, not every open question of the theory, and its subject index lists what each record is about.

Records: 9. By state: open 9, settled 0, succeeded 0, withdrawn 0.
By concern: theory 6, model 0, documents 1, machinery 2.

| Record | Title | Concern | State |
| --- | --- | --- | --- |
| `open:converter:baseline-calibration` | the baseline converter | theory | open |
| `open:deontic:binding` | the deontic's scope | theory | open |
| `open:explainer:claim-scope` | the companion explainer | documents | open |
| `open:measurement:construct-validity` | construct validity | theory | open |
| `open:paper:fixed-data-tables` | the paper's fixed data tables | machinery | open |
| `open:paper:typed-layer-checks` | the typed layer's checks | machinery | open |
| `open:profile:antichain-cause` | the antichain's cause | theory | open |
| `open:profile:narration-motivation` | the narration coordinate's motivation | theory | open |
| `open:stock:reservoir-presence` | the stock rival | theory | open |

## the baseline converter

Record `open:converter:baseline-calibration`, theory, open.

What value does the identification scheme's own method give the baseline converter, and does one global linear elasticity suffice for it or does it vary from node to node?

- About: paper object c:nu, `sem:converter:funded-gradient-to-rate`; paper object maint:ident, `sem:claim:will-identification`.
- Blocks: paper object op:vmoral, `sem:operator:moral-baseline`.
- Settled when: A calibration is posted by the identification scheme's own method with the bias direction the registry states, and the global linear form is either borne out by that measurement or replaced by the per-node alternative the shapes register prices.

## the deontic's scope

Record `open:deontic:binding`, theory, open.

Is the forcing the theory reads as deontic field-impervious, with susceptibility identically zero, rather than a pull saturated flat on the range tested?

- About: paper object deon:open, `sem:source-class:open-loop-forcing`; paper object deon:test, `sem:claim:one-sided-susceptibility-test`.
- Settled when: Only refutation settles it: a nonzero susceptibility slope across environments in a forcing the theory reads as deontic answers no, and no observation answers yes, because a zero slope is also what a saturated pull shows.

## the companion explainer

Record `open:explainer:claim-scope`, documents, open.

In which of the paper's degenerate corners, the restricted domains on which the full theory reduces to a simpler form, does each claim of the companion explainer hold, and which of its claims hold in none?

- About: file `documents/paper/explainer/explainer.tex`; paper label `tbl:corners`.
- Settled when: Every claim of the explainer names the corner in which the paper supports it, and every claim that holds in no corner is withdrawn from its text.

## construct validity

Record `open:measurement:construct-validity`, theory, open.

Are the anchored observables measurable at all, that is, can an instrument with established construct validity estimate them?

- About: paper object pr:scope, `sem:claim:verdict-model-scope`.
- Settled when: An instrument for the anchored observables is specified and its construct validity established, or an argument shows that no such instrument can exist.

## the paper's fixed data tables

Record `open:paper:fixed-data-tables`, machinery, open.

Do the paper's fixed data tables state what the theory graph states, object for object, tag for tag and register for register?

- About: record `dec:paper:authored-latex`; file `documents/paper/data/ftoo_manifest.tex`; file `documents/paper/data/ftoo_registers.tex`; file `documents/paper/data/ftoo_symbols.tex`; file `documents/paper/data/ftoo_symbols_historical.tex`; file `documents/paper/data/ftoo_index.tex`; paper label `app:ledger`.
- Settled when: Each table is generated from the theory graph under a byte gate after its fixed content has been compared with the graph, every difference has been resolved in the graph, and the table has left the paper's source list.

## the typed layer's checks

Record `open:paper:typed-layer-checks`, machinery, open.

Which of the typed layer's build-time refusals and warnings over grades, spends, branches and evidence keys become numbered rules over parsed unit mathematics, and what checking is lost for each that does not?

- About: record `dec:paper:authored-latex`; record `arg:paper:consequences`; file `documents/paper/ftoo_typed_tools.tex`; file `documents/paper/macros_overlap.tex`; file `documents/paper/econ_membrane.tex`; paper label `tbl:verify`.
- Rests on: open question `open:paper:fixed-data-tables`.
- Settled when: Every refusal and warning of the typed layer is a registered rule with its state and scope or a loss stated in the verification contract, and the typed-tools file has left the paper's source list.

## the antichain's cause

Record `open:profile:antichain-cause`, theory, open.

When two real lives are incomparable under the canonical partial order, what separates incomparability by type, which the order proves, from incomparability by ignorance, where the lives are merely unread?

- About: paper object P:order, `sem:claim:canonical-partial-order`; paper object P:anti, `sem:profile:antichain`; paper label `tbl:P:antichain`.
- Settled when: A criterion drawn from the lives' readings tells the two kinds of incomparability apart for the antichain's members, or an argument shows that no reading can tell them apart.

## the narration coordinate's motivation

Record `open:profile:narration-motivation`, theory, open.

What motivates typing the profile's narration coordinate as a node-private welfare-like axis?

- About: paper object P:nar, `sem:named:trait-narration`; paper object P:vec, `sem:profile:profile`.
- Blocks: paper object P:nonorm, `sem:claim:no-dimensionally-natural-norm`.
- Settled when: An argument or construction derives the node-private welfare-like typing of the narration coordinate, or a different typing replaces it.

## the stock rival

Record `open:stock:reservoir-presence`, theory, open.

Does maintenance draw on a reservoir whose time constant a release experiment can resolve, so that a released curve shows a knee?

- About: paper object maint:knee, `sem:claim:release-knee`.
- Blocks: paper object gen:will, `sem:foundation:will-package`.
- Settled when: Only a finding settles it: a release that shows a knee at the experiment's resolution answers yes, and each knee-free release only tightens the upper bound on the reservoir's time constant.

## Subjects

| Subject | Records |
| --- | --- |
| record `arg:paper:consequences` | `open:paper:typed-layer-checks` |
| record `dec:paper:authored-latex` | `open:paper:fixed-data-tables`, `open:paper:typed-layer-checks` |
| paper label `app:ledger` | `open:paper:fixed-data-tables` |
| paper object P:order, `sem:claim:canonical-partial-order` | `open:profile:antichain-cause` |
| paper object deon:test, `sem:claim:one-sided-susceptibility-test` | `open:deontic:binding` |
| paper object maint:knee, `sem:claim:release-knee` | `open:stock:reservoir-presence` |
| paper object pr:scope, `sem:claim:verdict-model-scope` | `open:measurement:construct-validity` |
| paper object maint:ident, `sem:claim:will-identification` | `open:converter:baseline-calibration` |
| paper object c:nu, `sem:converter:funded-gradient-to-rate` | `open:converter:baseline-calibration` |
| paper object P:nar, `sem:named:trait-narration` | `open:profile:narration-motivation` |
| paper object P:anti, `sem:profile:antichain` | `open:profile:antichain-cause` |
| paper object P:vec, `sem:profile:profile` | `open:profile:narration-motivation` |
| paper object deon:open, `sem:source-class:open-loop-forcing` | `open:deontic:binding` |
| paper label `tbl:P:antichain` | `open:profile:antichain-cause` |
| paper label `tbl:corners` | `open:explainer:claim-scope` |
| paper label `tbl:verify` | `open:paper:typed-layer-checks` |
| file `documents/paper/data/ftoo_index.tex` | `open:paper:fixed-data-tables` |
| file `documents/paper/data/ftoo_manifest.tex` | `open:paper:fixed-data-tables` |
| file `documents/paper/data/ftoo_registers.tex` | `open:paper:fixed-data-tables` |
| file `documents/paper/data/ftoo_symbols.tex` | `open:paper:fixed-data-tables` |
| file `documents/paper/data/ftoo_symbols_historical.tex` | `open:paper:fixed-data-tables` |
| file `documents/paper/econ_membrane.tex` | `open:paper:typed-layer-checks` |
| file `documents/paper/explainer/explainer.tex` | `open:explainer:claim-scope` |
| file `documents/paper/ftoo_typed_tools.tex` | `open:paper:typed-layer-checks` |
| file `documents/paper/macros_overlap.tex` | `open:paper:typed-layer-checks` |
