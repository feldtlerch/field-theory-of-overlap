# Architecture decision records · `guide:adr:index`

**Convention (What an ADR is here)** · `conv:adr:form`

An ADR freezes one coherent decision: the decision itself, the reasons, the alternatives declined with why, and the consequences accepted. Where a reason comes from experience, the record says so plainly — this project re-implements, with a fresh history, a design that converged over several private generations, and the lessons of that earlier implementation are carried here as arguments rather than as inherited code. An ADR is **accepted** when it lands reviewed and **superseded** when a later ADR names it; nothing here is draft.

**Convention (Labels)** · `conv:adr:labels`

Documents in this repository mint labels of the form `kind:area:slug` — a kind naming what the statement is (a record, a decision, an argument, a rule), an area naming its subject, and a slug unique within both. A label is minted once, at the statement it names, and cited in parentheses with single backticks; `docs/glossary.md` carries the project vocabulary the records use. Documents draw on the kind taxonomy's full breadth — a claim is minted under a result kind, an argument under an argument kind, a convention as a convention — because a statement minted under the wrong kind misstates what it is, and the kind's fit is part of what review judges.

**Register (The ADRs)** · `reg:adr:register`

|ADR|Decision|State|
|---|--------|-----|
|001|The authority is a closed typed graph|accepted|
|002|Four-axis status and declared coverage|accepted|
|003|Tags: human-facing handles bound to labels|accepted|
|004|The closed notation AST with two renderers and derived tiers|accepted|
|005|Statement units: one text per language, per-target displays|accepted|
|006|Generated artifacts are byte-gated; generate and check share one path|accepted|
|007|The audit runs on a public certified-calculation engine|accepted|
|008|Publication: fresh history and the severity gate|accepted|
|009|Process: signed reviewed bites, each layer with its validator|accepted|
|010|Validation rules form a registered, stated-scope rule set|accepted|
|011|Serialization is strict everywhere|accepted|
|012|The weld: equal address sets across a unit's texts|accepted|
|013|Workspace norms: edition, lints, toolchain|accepted|
