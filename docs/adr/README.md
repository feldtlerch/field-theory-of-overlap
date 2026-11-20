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
|004|The closed notation AST with two renderers and derived tiers|accepted|
|005|Statement units: one text per language, per-target displays|accepted; the paper pair is governed by 014; prose typography is governed by 021; progressive reading assembly is governed by 025|
|006|Generated artifacts are byte-gated; generate and check share one path|accepted; input-equivalent docs-only changes are governed by 018 and ordinary paper-pair reuse by 019|
|007|The audit runs on a public certified-calculation engine|accepted|
|008|Publication: fresh history and the severity gate|accepted|
|009|Process: signed reviewed bites, each layer with its validator|accepted|
|010|Validation rules form a registered, stated-scope rule set|accepted|
|011|Serialization is strict everywhere|accepted|
|012|The weld: equal address sets across a unit's texts|accepted|
|013|Workspace norms: edition, lints, toolchain|accepted|
|014|The paper pair: authored LaTeX, reproducible build, checked addressing|accepted; source authority, the statement-unit exception and rendering targets are governed by 016; layout selection is governed by 023 and 024|
|015|Open questions are records|accepted|
|016|Admission and native document renderings|accepted; retention of reader Markdown label comments is governed by 017; layout-pin clauses and passage-renewal acceptance are governed by 024; progressive ownership and quotation transfer are governed by 025|
|017|Screen rendering contracts|accepted; mathematics transport is governed by 022; named passage-renewal acceptance is governed by 024|
|018|Docs-only gate inherits document bytes through input equivalence|accepted|
|019|The paper pair reuses exact-input production across ordinary rounds|accepted; amends 006 for the paper pair|
|020|Meson and Ninja own the warm build graph while Cargo builds Rust|accepted|
|021|Reader characters in unit prose; target spellings per renderer|accepted; amends 005's prose-text clause|
|022|Mathematics transport in forge Markdown|accepted; amends 017's mathematics-dialect reservation|
|023|Site-local paper layout with two retained coordinates|accepted; its two-coordinate qualification is superseded by 024|
|024|Paper layout without source coordinates|accepted; supersedes the layout-pin clauses of 014 and 016; states who accepts changes: the project's owner approves theory changes, and independent review accepts all others|
|025|Identified progressive reading trees|accepted; amends 001, 002, 005 and 016|
|026|Warranted structural change|accepted; amends 001's identity-change mechanism while retaining historical meaning|
