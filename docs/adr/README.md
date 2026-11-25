# Architecture decision records · `guide:adr:index`

**Convention (What an ADR is here)** · `conv:adr:form`

An ADR freezes one coherent decision: the decision itself, the reasons, the alternatives declined with why, and the consequences accepted. Where a reason comes from experience, the record says so plainly — this project re-implements, with a fresh history, a design that converged over several private generations, and the lessons of that earlier implementation are carried here as arguments rather than as inherited code. An ADR is **accepted** when it lands reviewed and **superseded** when a later ADR names it; nothing here is draft.

**Convention (Labels)** · `conv:adr:labels`

Documents in this repository mint labels of the form `kind:area:slug` — a kind naming what the statement is (a record, a decision, an argument, a rule), an area naming its subject, and a slug unique within both. A label is minted once, at the statement it names, and cited in parentheses with single backticks; `docs/glossary.md` carries the project vocabulary the records use. Documents draw on the kind taxonomy's full breadth — a claim is minted under a result kind, an argument under an argument kind, a convention as a convention — because a statement minted under the wrong kind misstates what it is, and the kind's fit is part of what review judges.

**Register (The ADRs)** · `reg:adr:register`

|ADR|Decision|State|
|---|--------|-----|
|001|The authority is a closed typed graph|accepted; warranted structural change and historical meaning are governed by 026; reading identities are governed by 025|
|002|Four-axis status and declared coverage|accepted; reading lifecycle and availability are governed by 025|
|003|Tags: human-facing handles bound to labels|accepted; handle coverage and audit-independent addressing are governed by 016; reader Markdown label comments are governed by 017|
|004|The closed notation AST with two renderers and derived tiers|accepted; inherited definitions' referent prerequisite and pending presentation are governed by 032; bounded document-expression text is governed by 044|
|005|Statement units: one text per language, per-target displays|accepted; the paper pair is governed by 014; prose typography is governed by 021; progressive reading assembly is governed by 025; eligible native conversion's cross-target judgment is governed by 039; auxiliary paper mints are governed by 048|
|006|Generated artifacts are byte-gated; generate and check share one path|accepted; input-equivalent docs-only changes are governed by 018 and ordinary paper-pair reuse by 019|
|007|The audit runs on a public certified-calculation engine|accepted|
|008|Publication: fresh history and the severity gate|accepted|
|009|Process: signed reviewed bites, each layer with its validator|accepted; eligible native conversion's review timing is governed by 039|
|010|Validation rules form a registered, stated-scope rule set|accepted|
|011|Serialization is strict everywhere|accepted|
|012|The weld: equal address sets across a unit's texts|accepted|
|013|Workspace norms: edition, lints, toolchain|accepted|
|014|The paper pair: authored LaTeX, reproducible build, checked addressing|accepted; source authority, the statement-unit exception and rendering targets are governed by 016; layout selection is governed by 023 and 024; the complete-document release gate and complete-graph boundary are governed by 030; native use of the paper-scoped store is governed by 037|
|015|Open questions are records|accepted|
|016|Admission and native document renderings|accepted; retention of reader Markdown label comments is governed by 017; layout-pin clauses and passage-renewal acceptance are governed by 024; progressive ownership and quotation transfer are governed by 025; the complete-document release gate, complete-graph boundary and native-model boundary are governed by 030; retirement of residual authored-source machinery is governed by 036; native use of the paper-scoped store is governed by 037; the auxiliary paper-mint metadata route is governed by 048|
|017|Screen rendering contracts|accepted; forge Markdown mathematics transport is governed by 022 and the observed display thick-space spelling by 051; HTML mathematics delivery and font choice are governed by 042; named passage-renewal acceptance is governed by 024; the complete-document release gate is governed by 030; eligible native conversion's screen-review timing is governed by 039; transitional screen destinations are governed by 040|
|018|Docs-only gate inherits document bytes through input equivalence|accepted|
|019|The paper pair reuses exact-input production across ordinary rounds|accepted; amends 006 for the paper pair|
|020|Meson and Ninja own the warm build graph while Cargo builds Rust|accepted|
|021|Reader characters in unit prose; target spellings per renderer|accepted; amends 005's prose-text clause; inline-style authoring syntax is governed by 033|
|022|Mathematics transport in forge Markdown|accepted; amends 017's mathematics-dialect reservation; its observed display thick-space spelling is governed by 051; its reserved HTML-delivery question is governed by 042|
|023|Site-local paper layout with two retained coordinates|accepted; its two-coordinate qualification is superseded by 024|
|024|Paper layout without source coordinates|accepted; supersedes the layout-pin clauses of 014 and 016; states who accepts changes: the project's owner approves theory changes, and independent review accepts all others; eligible native conversion's review timing is governed by 039|
|025|Identified progressive reading trees|accepted; amends 001, 002, 005 and 016|
|026|Warranted structural change|accepted; amends 001's identity-change mechanism while retaining historical meaning; eligible native conversion's depth-1 warrant and review timing are governed by 039|
|030|Native renderings, complete graph and native model|accepted; establishes the complete-document release gate, complete-graph boundary and native-model boundary; amends 014, 016 and 017; relationship disposition at the complete-graph boundary is governed by 036; native use of the existing paper-scoped store is governed by 037; its transitional screen-placeholder clause is qualified by 040|
|032|Inherited notation before graph binding|accepted; qualifies 004's referent prerequisite and pending presentation while retaining 022's strict expanded screen contract; native support is governed by 041; bounded document-expression text, admission timing for the three additional inherited definitions and their registered-variance counts are governed by 044|
|033|Inline styles in unit prose|accepted; decides the authoring syntax reserved by 021; retains 022's mathematics boundary|
|036|Relationship dispositions at the complete-graph boundary|accepted; amends 030's relationship-checking clauses and 016's residual-machinery retirement clause|
|037|Native paper-tagged paragraphs|accepted; auxiliary visible handle placements and paper-key reader spans are governed by 048|
|038|Independent complete-document text comparison|accepted; line-end interpretation is governed by 043|
|039|Mechanical conversion and final residual review|accepted; amends 005, 009, 017, 024 and 026 for eligible native conversion; complete conversion checks and evidence retention are governed by 049|
|040|Explicit transitional destinations in a placeholder index|accepted; qualifies 017's missing-content, reference and placeholder clauses as amended by 030; extends 046's registered product check to screen destinations and content gaps; amends M055 and M065; section and subsection resolution is governed by 047; generated-register handling is governed by 054|
|041|Compared inherited support in native documents|accepted; qualifies 032's native sweep exception and embedding-removal condition, and amends M055, M056 and M057; the three additional compared flow declarations and their native sweeps are governed by 044|
|042|HTML mathematics transport and faces|accepted; amends 017's HTML-delivery and font-choice reservations for the admitted MathML profile|
|043|Source-decided line-end comparison|accepted; qualifies 038's line-end interpretation and extends M077|
|044|Text and references inside mathematics|accepted; qualifies 004 and 032's mathematical text representation and extends 041's compared support and native sweep; M054 and M055 govern checked unnumbered equations in TeX and Markdown, while HTML retains a block gap until display observation and admission|
|046|Registered HTML block gaps|accepted; the development product check inventories withheld HTML passages without changing release refusal; its registered-check and product-test sentences are replaced by 040; generated-register handling is governed by 054|
|047|Paper section and subsection destinations|accepted; extends 040's heading resolution and amends M055|
|048|Native reference forms and handle placements|accepted; visible auxiliary handles and their paper-origin evidence are admitted; forced-bare tags, silent handles, located-tag print-mode evidence and direct-reference counter-context forms await consumers|
|049|Complete conversion checks|accepted; qualifies 039's eligibility, retention sequence, capability, evidence, correction and completion clauses, and its qualifications of 017, 024 and 026|
|051|Thick spaces in forge Markdown displays|accepted; qualifies 017's pinned-command sentence and 022's display spelling for the observed dimensioned thick space|
|054|Generated gap registers|accepted; qualifies 046's registered-check clause as replaced by 040 for canonical generation of the two gap registers while retaining reviewable diffs and release refusal|
