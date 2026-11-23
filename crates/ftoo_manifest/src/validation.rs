use crate::{
    Awaiting, ClaimAudit, ENGINE_PIN, Edge, ManifestDraft, NodeId, NodeKind, NodePayload,
    RelationKind, ValidatedManifest, adapter_implemented,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

/// Permanent numbers in the manifest rule register.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RuleId {
    M001,
    M002,
    M003,
    M004,
    M005,
    M006,
    M007,
    M008,
    M009,
    M010,
    M011,
    M012,
    M013,
    M014,
    M015,
    M016,
    M017,
    M018,
    M019,
    M020,
    M021,
    M022,
    M023,
    M024,
    M025,
    M026,
    M027,
    M028,
    M029,
    M030,
    M031,
    M032,
    M033,
    M034,
    M035,
    M036,
    M037,
    M038,
    M039,
    M040,
    M041,
    M042,
    M043,
    M044,
    M045,
    M046,
    M047,
    M048,
    M049,
    M050,
    M051,
    M052,
    M053,
    M054,
    M055,
    M056,
    M057,
    M058,
    M059,
    M060,
    M061,
    M062,
    M063,
    M064,
    M065,
    M066,
    M067,
    M068,
    M069,
    M070,
    M071,
    M072,
    M073,
    M074,
    M077,
    M078,
    M079,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleState {
    Structural,
    Enforced,
    DeferredM2,
    DeferredExclusiveRoutes,
    DeferredPaperUnits,
}

/// Scope and deferral are part of the rule, not inferred from an empty domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleSpec {
    pub number: RuleId,
    pub statement: &'static str,
    pub scope: &'static str,
    pub state: RuleState,
    pub deferred_reason: Option<&'static str>,
}

pub const RULES: [RuleSpec; 77] = [
    RuleSpec {
        number: RuleId::M001,
        statement: "Input is valid UTF-8 without a BOM.",
        scope: "Every byte load.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M002,
        statement: "Decoded object keys are unique and input contains exactly one document.",
        scope: "Every object at every depth.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M003,
        statement: "Numeric tokens are integers and the wire schema is exactly integer 1.",
        scope: "Every numeric token at the byte boundary.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M004,
        statement: "Wire fields have exact required shapes and exclusive variant fields.",
        scope: "Every wire object and scalar.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M005,
        statement: "Identifiers, kinds, relations, guards, status, binding, capabilities, budgets, verdict classifications and Unconditional routes are closed; payload determines kind.",
        scope: "Every admitted Rust value.",
        state: RuleState::Structural,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M006,
        statement: "Node IDs are unique, payload kinds match identity, and descriptions are nonblank.",
        scope: "Every draft node.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M007,
        statement: "Every edge endpoint and coverage member resolves to a present node.",
        scope: "All references regardless of closure flags.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M008,
        statement: "Endpoint kinds satisfy the relation's own permitted pair table.",
        scope: "Edges with resolved, valid nodes.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M009,
        statement: "Edge tuples are unique and self edges obey the relation policy.",
        scope: "Every directed (relation, source, target, guard) record.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M010,
        statement: "Each acyclic relation has no cycle in its entire subgraph.",
        scope: "Edges passing endpoint and self-edge checks, including nodes outside the slice; multiplicity does not affect connectivity.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M011,
        statement: "Only maturity is authored; base status derives from identity/payload and AuditedStatus only through private checked-report construction.",
        scope: "Base and audited status APIs.",
        state: RuleState::Structural,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M012,
        statement: "Assertions equal their surface's derivation: unaudited projection or replay-checked audited report.",
        scope: "Every loaded status assertion.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M013,
        statement: "Every declared slice member is unique and is a claim; M007 resolves membership before this shape check.",
        scope: "Resolved members; shape precedes eligibility.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M014,
        statement: "GraphAdmission requires each slice claim Bound; RegistryAdmission requires transitive claim premises selected; ReportConsumption requires reached IDs equal the declared slice.",
        scope: "GraphAdmission, RegistryAdmission and ReportConsumption.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M015,
        statement: "Audit closure true requires every claim Bound to its implementation; ExplicitPartial and Complete satisfy the stated report predicates, including nonvacuity.",
        scope: "Every manifest, including an empty graph.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M016,
        statement: "Shared weighting compares paths of the same kind only.",
        scope: "Holding and covector payloads carrying weighting paths.",
        state: RuleState::DeferredM2,
        deferred_reason: Some("M2 must admit typed weighting paths and their consumer."),
    },
    RuleSpec {
        number: RuleId::M017,
        statement: "Reads-through is required only for field-anchored readings.",
        scope: "Field readings, not latent readings.",
        state: RuleState::DeferredM2,
        deferred_reason: Some("M2 must admit anchored readings and route checking."),
    },
    RuleSpec {
        number: RuleId::M018,
        statement: "Converters satisfy their declared input, conversion and output.",
        scope: "Converter declarations, not inferred derivation contracts.",
        state: RuleState::DeferredM2,
        deferred_reason: Some("M2 must admit converters and their checking consumer."),
    },
    RuleSpec {
        number: RuleId::M019,
        statement: "Indexed-space families and operations satisfy family-level typing at their node index.",
        scope: "Every admitted indexed family and operation.",
        state: RuleState::DeferredM2,
        deferred_reason: Some("M2 must admit indexed spaces and operation typing."),
    },
    RuleSpec {
        number: RuleId::M020,
        statement: "Every Bound claim has exactly its designated implementation and every implementation exactly one present Bound claim; admitted contracts pass their numbered consumers.",
        scope: "Bound claims and the executing registry.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M021,
        statement: "Every symmetric edge has its reverse with the same relation and guard.",
        scope: "Edges passing M007, M008 and self-edge policy; compatibility only.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M022,
        statement: "Fixed producers have exactly their declared dependencies; Certified premises unlock execution and target witnesses contain full coefficient dependencies.",
        scope: "Registry and execution consumers.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M023,
        statement: "Every Bound claim has its designated reviewed budget; all records are used, pin/profile agree and claim work shares one budget.",
        scope: "Registry and execution consumers.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M024,
        statement: "Supported means adapter-implemented at this pin solely from the table; unsupported requirements never execute and Awaiting supported contracts is refused.",
        scope: "Registry and execution consumers.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M025,
        statement: "Exactly one outcome exists per claim, none for quantities; claim, producer, revision, pin, budget and semantic snapshot match.",
        scope: "Report domain and semantic provenance.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M026,
        statement: "Certified/Refuted consume matching replay-checked evidence; only typed limits yield exhaustion and invalid construction never refutes.",
        scope: "Private receipt construction and verdict interpretation.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M027,
        statement: "Each execution has a fresh store and one budget; gated evidence is independent of measurements and previous caches.",
        scope: "Fresh execution and complete gated report reproducibility.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M028,
        statement: "Every unselected or blocked claim has its exact derived deferral; uncertified prerequisites cannot promote dependents.",
        scope: "Total report blockers and Certified-premise scheduling.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M029,
        statement: "Alternative routes execute alone and combine by a reviewed rule.",
        scope: "Registry and execution consumers.",
        state: RuleState::DeferredExclusiveRoutes,
        deferred_reason: Some("Exclusive routes await their first claim and isolation consumer."),
    },
    RuleSpec {
        number: RuleId::M030,
        statement: "Tag targets and spellings are fixed; no lifecycle operation can rebind or rename a tag or reactivate a retired tag.",
        scope: "Every public addressing value and operation.",
        state: RuleState::Structural,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M031,
        statement: "Tag spellings are nonempty, contain no whitespace or control character, and are unique by exact UTF-8 bytes.",
        scope: "CatalogAdmission over theory and documentary descriptors, including retired handles in both domains.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M032,
        statement: "History admits every permanent identity and tag exactly once, in a contiguous event order, with identity admission before tag admission.",
        scope: "HistoryAdmission over the whole ledger, not just present nodes.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M033,
        statement: "History records carry nonblank reasons, valid authority-record labels and nonblank original identity formulations.",
        scope: "HistoryAdmission for every event's metadata.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M034,
        statement: "Retirement is terminal, follows admission, and a retired identity cannot remain in the addressed graph.",
        scope: "HistoryAdmission and StoreConstruction.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M035,
        statement: "The live, retired and out-of-slice stores partition every admitted tag according to retirement and declared-slice dependency support.",
        scope: "StoreConstruction over the full catalog.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M036,
        statement: "Every permanent theory identity has a tag, and the theory branch of resolution returns its bound NodeId in a Theory result only for a live exact handle, otherwise its exact named refusal.",
        scope: "Completed theory catalog coverage and the theory branch of the common resolution consumer.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M037,
        statement: "A projection preserves every graph record and fixed authority assertion, including complete budgets, bindings, descriptors, history and store event references.",
        scope: "Projection loading of fixed assertions and generation/check semantic comparison against Rust authority.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M038,
        statement: "Generation and check share one canonical writer, and a generated projection must equal its regenerated bytes.",
        scope: "The generated manifest artifact and its shared generation/check path.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M039,
        statement: "Unit metadata carries a well-formed label unique within the admitted unit corpus and references a declared document language contract; document names obey the admitted grammar.",
        scope: "Statement-unit metadata and document declaration names in one corpus admission.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M040,
        statement: "Documents declare nonempty distinct language sets, a main document is declared and requires English alone, and each unit has exactly one nonblank authored text per required language and no text for an undeclared language; language tokens obey the admitted grammar. A document's optional targets list is a nonempty set of distinct values from fragment, markdown, latex and html, and defaults to markdown and latex. A document's optional paper-context is exactly paper; omission supplies no paper-scoped resolver.",
        scope: "Document language, target and paper-context declarations and all associated statement-unit texts.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M041,
        statement: "Statement texts use the admitted insertion syntax and reference only their unit's shared nonblank TeX math entries, each of which is used; ordinary prose and math entries use only the admitted characters and fragment syntax, and math names obey the local-name grammar. Bibliography citation insertions retain their parenthetical or textual form and an ordered nonempty list of distinct keys in the grammar a:[A-Za-z0-9-]+; a citation inside a style refuses. Inline styles use the four semantic insertion kinds, retain located child identities, and refuse nesting, empty bodies, boundary whitespace, adjacent same-kind runs, a run containing its own closer, and styled mathematics.",
        scope: "Every authored statement text, shared math entry and local math insertion.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M042,
        statement: "Every tag-facing statement-text occurrence first resolves through the supplied AddressBook's live resolver. Only an unknown permanent spelling in a document declaring paper context may fall back to the checked paper-scoped lookup; retired and out-of-slice refusals and documentary-target refusals are terminal. The fallback accepts only paper or alias rows of kind node or evidence; admitted and candidate rows refuse. Resolution preserves the target domain, printed spelling, source key and evidence without permanent admission, or refuses with its unit, language and site.",
        scope: "All tag insertions in every admitted language text.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M043,
        statement: "Every label has one mint across the permanent Rust identity catalog, Markdown mint occurrences and admitted unit metadata; Markdown citations resolve within the Markdown mint domain.",
        scope: "The complete supplied Markdown corpus, every permanent NodeId and every admitted unit metadata mint.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M044,
        statement: "Every Rust-held authority-record citation resolves to exactly one Markdown mint.",
        scope: "Every history event in the supplied resolved corpus's AddressBook, including admission and retirement records.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M045,
        statement: "Unit texts never mint labels; bare label spans cite uniquely minted labels and doubled spans mention without harvesting or resolution.",
        scope: "Every admitted statement text in every required language, with citations resolved in the joint label domain.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M046,
        statement: "Within each statement unit, every language has equal addressed tag sets and equal cited label sets, compared separately from the shared metadata mint and independently of position or multiplicity.",
        scope: "All language texts of each resolved unit after joint label and citation admission.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M047,
        statement: "Notation expressions use only the closed registry, exact signatures, bounded literal grammar and scoped constructor parameters, and every formal is used, within declared notation limits. Inherited signatures and decoded bodies obey the same grammar and limits, with no checking-effect fields.",
        scope: "Every primitive descriptor, definition body and submitted notation expression.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M048,
        statement: "Definitions have their kind's signature shape and any backing equals their object; stems obey the reserved alphabetic grammar and generated names are unique; every permanent object has exactly one canonical symbol; aliases preserve that object and terminate at its canonical symbol; all definition dependencies are acyclic. An inherited definition has a checked signature and decoded body but no theory object or backing; an identical fully expanded body with an object-bound canonical symbol or a dependency on a non-inherited definition in its own body refuses admission.",
        scope: "The complete notation definition catalogue, including absent objects and unused definitions.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M049,
        statement: "Symbol tiers are derived from the supplied graph, an accepted retained audit context where supplied, and transitive symbol uses; an alias takes the lesser of its own binding tier and its target tier, retaining every pending reason. An inherited definition without a theory object is pending with a missing-binding reason, which every definition using it retains; generated notation-status evidence lists every inherited definition and its reason.",
        scope: "Base checked notation catalogues, accepted retained notation views and all exposed symbol-status values, including inherited missing bindings, transitive uses and generated/notation-status.json.",
        state: RuleState::Structural,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M050,
        statement: "Generated notation declarations refuse collisions in the consuming typesetting context. Temporary inherited declarations used by the sweep and expression validation refuse collisions in that same checked KaTeX context.",
        scope: "Every emitted notation declaration and temporary inherited declaration used in the checked KaTeX context.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M051,
        statement: "Both notation forms derive from the same checked AST with explicit token boundaries and within the output limit; full Unicode expansion contains no project commands and every returned expression passes the pinned KaTeX grammar. For inherited definitions instead, the paper form invokes the existing checked source declaration by name, whose body is source-compared with the admitted AST, and the strict screen form derives from that AST; pending marks and visible pending explanations are omitted only when every pending reason is an inherited missing binding.",
        scope: "Generated declaration bodies, macro invocations and fully expanded notation expressions, including named inherited paper invocations and their checked source-comparison exception.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M052,
        statement: "A generated sweep typesets every emitted notation command with its declared arguments and checks its fully expanded counterpart. The KaTeX sweep includes temporary declarations and fully expanded strict screen forms for inherited definitions.",
        scope: "The complete emitted command inventory, including aliases, constructors, pending symbols and temporary inherited declarations in KaTeX.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M053,
        statement: "Every shared math source parses completely into the closed notation AST within the assembly math bounds, resolves every command with its declared syntax, and passes both rendering consumers with all authored use sites retained.",
        scope: "All math entries of every welded unit consumed by assembly.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M054,
        statement: "Every declared document has one ordered skeleton, every skeleton names a declared document, and every admitted unit occurs exactly once in its associated document's skeleton within the assembly bounds. A quotation child names an existing unit owned by another document with text in every language the quoting document requires, and adds no membership. A paragraph child is an owning prose position, not a heading, refrain or quotation.",
        scope: "The complete skeleton and admitted unit corpora, including heading positions.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M055,
        statement: "Markdown and LaTeX emission preserves the checked block and occurrence inventory, uses the matching notation and addressing context, refuses every unavailable runtime tag at its site, and generated documents contribute no Markdown mint or documentary citation. Bibliography citations resolve ordered keys against the stock paper bibliography only where their entries are unchanged by the paper bibliography derivation, with the refused key set pinned to that derivation; TeX recovery preserves their command form and ordered keys, Markdown and HTML recovery preserve their printed labels and destinations, and each screen has one source-derived entry at the planned References position for each cited key; a citation without a screen, a missing, extra, reordered or changed citation link or entry, or a changed References position refuses. Each cited bibliography key matches exactly one article entry ignoring key case. That entry admits only braced author, title, journal, volume, number, pages, year and doi fields, one literal author with an ASCII-letter surname, a four-digit year, and no TeX markup or font ligature spellings except a double hyphen in pages normalized to an en dash; other shapes refuse. A bibliography containing a percent sign or an @comment command refuses. Every other regular entry must yield a braced author and four-digit year for the whole-bibliography surname-and-year collision check, or the citation refuses; equal surname and year refuse. Styled runs preserve ordered boundaries and child identities through TeX and Markdown recovery; HTML recovery compares decoded prose, ordered style boundaries, bibliography citation kinds, link labels and destinations, ordered end-matter entries, coverage and gaps against checked plan fields, and embedded font bytes, face declarations and licence texts against pinned loaded faces and retained licence files; HTML emits screen documents with whole pinned font files and bibliography citations linked to planned References entries; a paragraph or refrain whose inline content has no HTML emitter becomes a plan-recorded, unlinked notice at that block position, recovered by position and wording, and a withheld quotation shows the notice without a link; an emitted quotation requires an emitted owner destination and refuses at the quotation site otherwise; release checks refuse HTML block gaps; missing screen plans, unrealizable coverage, display bodies, and references or unrealizable inline content in headings refuse. Direct inline mathematics in paragraph bodies is admitted only under the bounded exact-source MathML profile (`rec:adr:042-html-mathematics`); mathematics outside that profile has no HTML emitter and is included in the plan-recorded notice and release gap (M065). Mathematical presentation derives from each admitted run's strict expanded source; recovery compares its source annotation with that run's planned source, its visible tree with the prepared presentation for that occurrence, and its face declaration and embedded bytes with the pinned mathematics face; an adapter fault, unsupported output for an admitted source, a missing or mismatched mathematics face file, or a mismatch of a recovered mathematical occurrence refuses at the mathematical use site. The stylesheet line, face declaration, and font and licence inventories refuse at the coverage site. Markdown emits styles only in observed faithful contexts and refuses unsupported kinds or contexts, a punctuation inner edge whose outer neighbour is present and is neither ASCII whitespace nor ASCII punctuation, and direct mathematics, style, tag or citation adjacency at the style source site. Generated coverage and pending object names require a unique live tag. Every reader-document documentary citation requires a unique live tag for its cited label and retains that label in its cite comment; literal semantic-label tokens are refused in reader text. The only paper-key exception is the exact planned emitted spelling span of a checked paper tag run whose printed spelling equals its own source key; the same bytes in text, titles, hrefs or ids remain refused. Markdown mathematics is emitted only in admitted observed contexts, with a body that satisfies the delimiter and citation boundary, and refuses an unobserved context, forbidden body character or direct tag/citation, mathematics or code-mention adjacency at the mathematical use site before sizing or emission. A fragment target emits each non-heading position in its document as one TeX macro line: a plain-text refrain or quotation, or a paragraph containing admitted text, checked paper tags, bibliography citations, plain-text emphasis runs and mathematics invoking one of the compared inherited commands covered by the native sweep (comovfield, willvirtue or gensym); readout remains refused until its mathematics-alphabet context is decided; other bodies refuse. Paragraph target spellings preserve decoded characters and refuse merging hyphen/em-dash runs in plain text and emphasis or text requiring braced, backslash or multiline output. Each declared fragment replaces exactly one unique matching marker line in its declared source; malformed, repeated, undeclared or absent markers, role disagreements and authored duplicates of a converted body refuse. Every generated artifact path is unique. A selected target or role without an emitter refuses; Markdown quotations require the checked owner destination, and LaTeX quotations remain refused.",
        scope: "Every selected document target, its declared fragment source slots and generated notation and addressing support.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M056,
        statement: "Every generated LaTeX document and every derived notation sweep case except the readout inherited definition pass the pinned native document context with checked dependency integrity. Readout remains outside that native sweep until its mathematics-alphabet context is decided. Inherited paper commands retain checked source comparison and their screen forms retain the strict KaTeX sweep.",
        scope: "Prepared LaTeX documents, their actual preamble and support files, including the compared inherited support files, and every derived notation sweep case except the readout inherited definition.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M057,
        statement: "Document generation and check share one preparation and writer, and the complete generated document and support inventory equals its regenerated bytes. That inventory includes generated/notation-status.json and generated/native-paper-support.tex.",
        scope: "Generated Markdown, LaTeX and HTML documents, notation.tex, notation-status.json, native-paper-support.tex and addressing.tex.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M058,
        statement: "Every display has one existing unit owner, exactly one admitted body per target and required language, every body occurrence resolves under that unit's checked context, and a display unit occurs only as a skeleton leaf.",
        scope: "All display companion records and every cell or figure-label occurrence.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M059,
        statement: "Within each display, addressed tag sets and cited label sets are separately equal across all target and language bodies, retaining every occurrence independently of order and multiplicity.",
        scope: "All display bodies after complete body reference resolution.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M060,
        statement: "Documentary handles have immutable bindings to uniquely minted non-theory labels; their ordered provenance ledger admits each handle once and permits only terminal retirement; label-shaped documentary handles are refused and each documentary label has at most one live handle throughout replay; retained target and authority mints resolve, derived live and retired stores partition all admitted documentary handles, and resolution returns the bound documentary label only for a live exact handle. Target labels use the unit-label grammar or a name in the paper's label-mint inventory; authority labels always use the unit-label grammar.",
        scope: "The complete documentary tag registry, its joint-mint binding context and common-resolver documentary branch, including unused and retired handles.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M061,
        statement: "Retained audit bytes bind the exact current projection, audit/codec contract revision and linked engine source revision, and no retained assertion is accepted before complete gated re-derivation and byte equality.",
        scope: "Strict retained-report admission, provenance, bounds, canonical equality and every retained evidence context supplied to a consumer.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M062,
        statement: "Report generation and check use one canonical writer; a release claiming retained audit evidence requires a stable reread accepted report and matching generated artifacts.",
        scope: "Retained-report file lifecycle, committed report byte gate and release-retention consumer.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M063,
        statement: "Paper-scoped tag pairs are checked for uniqueness, grammar, exact source coverage and collision with both permanent registries; a changed row surfaces through the byte-input receipt and PDF byte gate. Confirmed admissions preserve immutable bindings and name their confirmation records. Aliases resolve to paper spellings and retain their own origin and their target's declaration origins; exact and case-insensitive reserved-name collisions and overlapping candidate ownership refuse. Each paper target-evidence row names a unique key and an existing declared source file containing that whole key; its file origin joins native and Markdown mint evidence without duplicate ownership and without entering strict-unit scope. Native paper resolution consumes the same Python-checked bindings through a strict, bounded hand-off with complete input digests and retained source evidence; malformed hand-offs, checker failures and changed inputs refuse. Check-only inputs invalidate document checking without becoming PDF byte inputs.",
        scope: "The paper's tag lookup with paper spellings, admissions, aliases and a provisional list, source declarations, permanent registries and reserved histories; consumed by paper preparation, documentary admission and native paper unit admission, label integration, welding, document assembly and recovery.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M064,
        statement: "Every rendered object identity resolves through an admitted, paper-scoped or explicit provisional binding to a two-part spelling; missing bindings refuse every build, and provisional bindings refuse release.",
        scope: "Expanded paper identity output, shipped occurrence ledgers and rendered-text leak checking; enforced by reader-hooks.tex, tools/paper/identities.py and tools/paper/ledger.py through the support-required paper gate. Number-only and family locators do not mint identities.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M065,
        statement: "Paper release requires no provisional printed identity, acceptance matching all nineteen final self-description passages, and an equal PDF pair and input receipt.",
        scope: "The paper check --release consumer conjoins existing identity, acceptance, HTML block gap and byte checks and names the failing part in one release refusal. Page acceptance and publication permission remain separate.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M066,
        statement: "Open-question states, concerns and reference kinds are closed sets; a settled, succeeded or withdrawn record cannot be constructed without its settlement references and answer, its successors, or its reason; admitted records are immutable, so no operation reopens a closed record.",
        scope: "Every admitted open-question value.",
        state: RuleState::Structural,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M067,
        statement: "The open-question store is one strict TOML document of schema 1, at most 1 MiB and 4096 records, whose records carry exactly the fields their state requires, with no empty optional list and no repeated list entry; labels have kind open and are unique and in label order; titles, questions, settling criteria, answers and withdrawal reasons are nonblank single-line sentences without backtick or vertical bar, a title has no terminal punctuation, a question ends with a question mark and every other text with a full stop; titles are unique and questions are unique after whitespace normalization.",
        scope: "registry/open-questions.toml, every record in every state.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M068,
        statement: "Labels of kind open are minted only by the open-question store: no Rust identity, Markdown mint, unit metadata, paper label-mint key or paper tag target uses the kind, and no store label has a second mint in the joint inventory; store labels do not yet enter any citation domain.",
        scope: "The joint label domain extended by the open-question store, and the paper's label-mint keys and tag targets.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M069,
        statement: "Every typed reference resolves in its own domain: node among permanent graph identities, rule in the register, unit in the admitted unit corpus, record among Markdown mints, paper among the paper's tag targets and label-mint keys, file as an existing regular repository file reached without symlink or parent step, open in the store; about is nonempty, blocks names no open record, and no record refers to itself.",
        scope: "The about, blocks, rests-on, settled-by and succeeded-by references of every record.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M070,
        statement: "Rests-on and succeeded-by are acyclic over the store; an open record rests only on open records; a settlement names only node, rule, unit, record or paper references; a succession names only open records or graph nodes.",
        scope: "The whole open-question store.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M071,
        statement: "Register generation and check share one writer; generated/open-questions.md equals its regenerated bytes, its counts are generated from the store, and it contains no Markdown mint span and no parenthesised label span.",
        scope: "The generated open-question register and its shared generate and check path.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M072,
        statement: "The paper's debts table is generated from the open-question store: one row per record the paper cites, its state rendered from the record, and no authored row.",
        scope: "The paper's debts table and its generator.",
        state: RuleState::DeferredPaperUnits,
        deferred_reason: Some(
            "The paper's appendices are authored LaTeX under ADR 014; the table is generated once they are statement units.",
        ),
    },
    RuleSpec {
        number: RuleId::M073,
        statement: "No Rust theory identity, Markdown mint, unit metadata, paper label-mint key, paper tag target or documentary admission target uses label kind read or audit; those kinds are kept for reading declarations and derived audit identities.",
        scope: "The joint mint inventory, the paper's label-mint keys and tag targets, and the label field of documentary admission events.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M074,
        statement: "Constructed reading names satisfy the semantic-label grammar with the reserved reading kind; identifiers and names are unique, specifications match their fixed declarations, and each quotation names an allocated owning occurrence in another reading.",
        scope: "Every constructed reading declaration.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M077,
        statement: "Requested paper-pair comparisons read captured PDFs in full through the recorded extractor, retain every raw page and classify every character under the stated profile and normalization rules. They compare complete canonical document streams and ordered bookmark titles and corroborate the declared paragraph with each side's captured source of that occurrence at its checked inclusion place. Only verified folios, whitespace, the listed presentation ligatures and source-decided line-end interpretation change extracted text. Line-end hyphens, en dashes, em dashes and slashes are retained as events before interpretation. An unchanged event may bypass interpretation only with an identical raw line, the same aligned token position and identical enclosing source bytes; an unidentified legacy interval may use the stricter unchanged-residual proof stated in ADR 043, never an offset-only match. Every other event requires unique source correspondence on each side independently; only a source-confirmed discretionary hyphen is removed, and authored marks and separators are retained. Complete comparison includes all text outside the corroborated paragraph. Checked bare paper tags retain the measured extracted small-cap spelling without general case folding, and correspondence refuses a mismatch; decoded paragraph text also compares their semantic spellings and targets. Outside decoded paragraph text, case inside small capitals is compared as extracted, so a source case change there can compare equal. Invalid or changed inputs, unavailable or ambiguous extraction, unsupported profiles or syntax, invalid furniture, absent or ambiguous selected-paragraph correspondence and exceeded correspondence bounds refuse. Unequal canonical content or bookmark titles reports differences. A difference that holds without the unresolved events (bookmark titles, source/body or source/tags, or a document stream with no unresolved events) reports different. The witness decides only otherwise. If only unresolved line-end coverage prevents complete interpretation, deleting the four event marks and whitespace everywhere from both streams may prove different when the resulting witnesses are unequal; otherwise the verdict is refused. Unresolved events are retained as coverage and never authorize equal. Changed locators, reflow at other break points and moved floats remain stated coverage limits, not permission to erase content, and can report differences without changed words. Equality attests only these comparisons, not paragraph boundaries, mathematical structure, checking effects, appearance or acceptance.",
        scope: "Explicit complete-paper-and-explainer comparison requests using the A4 plain-folio profile, a declared plain paragraph with checked bare paper tags and the bounded source-decided line-end grammar; complete raw and canonical streams, ordered bookmark titles, per-side captured source and unchanged-event/residual evidence. No automatic release or conversion acceptance, no derived-locator exclusion and no general legacy TeX decoder.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M078,
        statement: "Each inherited source declaration has the admitted signature, decoded visual body and complete dependency closure, with named quantity, metric and registered-variance interfaces unchanged. Source syntax and its checked expression form a reviewed pairing, and each inherited definition has exactly one compared source declaration site among the recognised declaration and redefinition forms in the three named source files. Registered-variance declaration sites for each inherited name across those files equal its recorded count.",
        scope: "Every admitted inherited declaration in documents/paper/data/ftoo_symbols.tex, documents/paper/data/ftoo_symbols_historical.tex and documents/paper/data/ftoo_registers.tex, including its checked source bytes and declared checking interface; declaration sites in other files are outside this comparison.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M079,
        statement: "Each excerpt occurrence is declared once in the one source scope record that names its paper source, with its document, unit and role; a second record for one source, a source outside the closed source list, an unknown document or role, or a repeated occurrence refuses, and the source holds exactly one marker line for each recorded unit (M055). Excerpt skeleton uses are ordered by the closed source list (documents/paper-build/sources.tsv) and then by marker line; document generation writes the two excerpt skeletons, and document checking refuses a committed excerpt skeleton whose uses differ before skeleton admission. Excerpt declarations remain authored and must equal the fixed excerpt template's required languages, targets and paper context. Document generation emits one inclusion list, checked byte for byte with the other generated documents; paper preparation consumes only that list and refuses a malformed row.",
        scope: "Source scope records, excerpt document declarations and skeletons, and checked paper inclusions.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
];

/// Original draft positions, retained separately from semantic sorting.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DraftLocation {
    Node(usize),
    Edge(usize),
    Slice(usize),
    Coverage,
    Binding(usize),
    Budget(usize),
    Capability(usize),
    Verdict(usize),
    Provenance,
    Cycle {
        relation: RelationKind,
        edges: Vec<usize>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub stage: AdmissionStage,
    pub rule: RuleId,
    /// Edge-scoped IDs retain source/target order; other IDs use semantic order,
    /// except cycles which retain their concrete traversal path order.
    pub ids: Vec<NodeId>,
    pub edge: Option<Edge>,
    pub location: DraftLocation,
    pub explanation: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationErrors {
    diagnostics: Vec<Diagnostic>,
}
impl ValidationErrors {
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}
impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diagnostic in &self.diagnostics {
            writeln!(
                f,
                "{:?} at {:?}: {}",
                diagnostic.rule, diagnostic.location, diagnostic.explanation
            )?;
        }
        Ok(())
    }
}
impl Error for ValidationErrors {}

fn report(
    diagnostics: &mut Vec<Diagnostic>,
    rule: RuleId,
    ids: Vec<NodeId>,
    edge: Option<Edge>,
    location: DraftLocation,
    explanation: String,
) {
    diagnostics.push(Diagnostic {
        stage: AdmissionStage::GraphAdmission,
        rule,
        ids,
        edge,
        location,
        explanation,
    });
}

/// Validate in dependency stages without inventing kinds for missing endpoints.
/// Node admission precedes resolution; endpoint contracts, tuple/self checks and
/// slice shape precede symmetry, cycles and eligibility. M021 depends on M008:
/// an invalid endpoint suppresses symmetry for that edge. Independent defects
/// accumulate; all diagnostics sort by rule, semantic tuple/identity, location.
///
/// # Errors
/// Returns deterministic diagnostics for every reached enforced-rule refusal.
pub fn validate(draft: ManifestDraft) -> Result<ValidatedManifest, ValidationErrors> {
    let mut diagnostics = Vec::new();
    let nodes = check_nodes(&draft, &mut diagnostics);
    let edges = check_edges(&draft, &nodes, &mut diagnostics);
    check_coverage(&draft, &nodes, &mut diagnostics);
    check_relations(&edges, &mut diagnostics);
    for (index, node) in draft.nodes.iter().enumerate() {
        if nodes.get(&node.id) == Some(&Some(NodeKind::Claim))
            && let NodePayload::Claim {
                audit: ClaimAudit::Awaiting(Awaiting::Capability(id)),
                ..
            } = node.payload
            && adapter_implemented(id, ENGINE_PIN)
        {
            report(
                &mut diagnostics,
                RuleId::M024,
                vec![node.id],
                None,
                DraftLocation::Capability(index),
                "cannot await an implemented adapter".into(),
            );
        }
    }
    diagnostics.sort_by(|a, b| {
        (a.rule, a.edge, &a.ids, &a.location, &a.explanation).cmp(&(
            b.rule,
            b.edge,
            &b.ids,
            &b.location,
            &b.explanation,
        ))
    });
    if diagnostics.is_empty() {
        Ok(ValidatedManifest::from_checked(draft))
    } else {
        Err(ValidationErrors { diagnostics })
    }
}

// None retains presence while withholding a kind when node admission failed.
fn check_nodes(
    draft: &ManifestDraft,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<NodeId, Option<NodeKind>> {
    let mut counts = BTreeMap::new();
    for node in &draft.nodes {
        *counts.entry(node.id).or_insert(0_usize) += 1;
    }
    let mut nodes = BTreeMap::new();
    for (index, node) in draft.nodes.iter().enumerate() {
        let mut valid = true;
        for (failed, reason) in [
            (counts[&node.id] != 1, "identity occurs more than once"),
            (
                node.id.expected_kind() != node.payload.kind(),
                "payload kind disagrees with identity",
            ),
            (
                node.payload.description().trim().is_empty(),
                "description is blank",
            ),
        ] {
            if failed {
                valid = false;
                report(
                    diagnostics,
                    RuleId::M006,
                    vec![node.id],
                    None,
                    DraftLocation::Node(index),
                    format!("{}: {reason}", node.id.semantic_name()),
                );
            }
        }
        nodes.insert(node.id, valid.then_some(node.payload.kind()));
    }
    nodes
}

fn check_edges(
    draft: &ManifestDraft,
    nodes: &BTreeMap<NodeId, Option<NodeKind>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<Edge, usize> {
    let mut counts = BTreeMap::new();
    for edge in &draft.edges {
        *counts.entry(*edge).or_insert(0_usize) += 1;
    }
    let mut admitted = BTreeMap::new();
    for (index, edge) in draft.edges.iter().enumerate() {
        let location = DraftLocation::Edge(index);
        let mut missing: Vec<_> = [edge.source, edge.target]
            .into_iter()
            .filter(|id| !nodes.contains_key(id))
            .collect();
        missing.sort();
        missing.dedup();
        for id in missing {
            report(
                diagnostics,
                RuleId::M007,
                vec![id],
                Some(*edge),
                location.clone(),
                format!("endpoint {} is absent", id.semantic_name()),
            );
        }
        let self_refused = edge.source == edge.target && !edge.relation.contract().allow_self;
        for (failed, reason) in [
            (counts[edge] != 1, "duplicate edge tuple"),
            (self_refused, "self edge is forbidden"),
        ] {
            if failed {
                report(
                    diagnostics,
                    RuleId::M009,
                    vec![edge.source, edge.target],
                    Some(*edge),
                    location.clone(),
                    reason.into(),
                );
            }
        }
        if let (Some(Some(source)), Some(Some(target))) =
            (nodes.get(&edge.source), nodes.get(&edge.target))
        {
            if !edge
                .relation
                .contract()
                .endpoints
                .contains(&(*source, *target))
            {
                report(
                    diagnostics,
                    RuleId::M008,
                    vec![edge.source, edge.target],
                    Some(*edge),
                    location,
                    format!(
                        "{} does not admit {source:?} -> {target:?}",
                        edge.relation.name()
                    ),
                );
            } else if !self_refused {
                // Multiplicity is already refused; a unique semantic projection
                // still permits independent symmetry and cycle checks.
                admitted.entry(*edge).or_insert(index);
            }
        }
    }
    admitted
}

fn check_coverage(
    draft: &ManifestDraft,
    nodes: &BTreeMap<NodeId, Option<NodeKind>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut counts = BTreeMap::new();
    for id in &draft.coverage.slice {
        *counts.entry(*id).or_insert(0_usize) += 1;
    }
    for (index, id) in draft.coverage.slice.iter().enumerate() {
        let location = DraftLocation::Slice(index);
        let Some(kind) = nodes.get(id) else {
            report(
                diagnostics,
                RuleId::M007,
                vec![*id],
                None,
                location,
                format!("slice member {} is absent", id.semantic_name()),
            );
            continue;
        };
        let Some(kind) = kind else {
            continue;
        };
        if counts[id] != 1 || *kind != NodeKind::Claim {
            report(
                diagnostics,
                RuleId::M013,
                vec![*id],
                None,
                location,
                format!("{} must occur once and be a claim", id.semantic_name()),
            );
        } else if draft.nodes.iter().any(|n| {
            n.id == *id
                && matches!(
                    n.payload,
                    NodePayload::Claim {
                        audit: ClaimAudit::Awaiting(_),
                        ..
                    }
                )
        }) {
            report(
                diagnostics,
                RuleId::M014,
                vec![*id],
                None,
                location,
                format!(
                    "{} awaits AuditBinding and is not audit-eligible",
                    id.semantic_name()
                ),
            );
        }
    }
    if draft.coverage.closed_world.audit
        && draft.nodes.iter().any(|n| {
            nodes.get(&n.id) == Some(&Some(NodeKind::Claim))
                && matches!(
                    n.payload,
                    NodePayload::Claim {
                        audit: ClaimAudit::Awaiting(_),
                        ..
                    }
                )
        })
    {
        report(
            diagnostics,
            RuleId::M015,
            vec![],
            None,
            DraftLocation::Coverage,
            "audit closure requires every present claim Bound".into(),
        );
    }
}

fn check_relations(edges: &BTreeMap<Edge, usize>, diagnostics: &mut Vec<Diagnostic>) {
    for (edge, index) in edges {
        if edge.relation.contract().symmetric && !edges.contains_key(&edge.reverse()) {
            report(
                diagnostics,
                RuleId::M021,
                vec![edge.source, edge.target],
                Some(*edge),
                DraftLocation::Edge(*index),
                format!(
                    "missing reverse {}: {} -> {} with {:?}",
                    edge.relation.name(),
                    edge.target.semantic_name(),
                    edge.source.semantic_name(),
                    edge.guard
                ),
            );
        }
    }
    for relation in RelationKind::ALL {
        if !relation.contract().acyclic {
            continue;
        }
        let mut adjacency: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
        for edge in edges.keys().filter(|edge| edge.relation == relation) {
            adjacency
                .entry(edge.source)
                .or_default()
                .insert(edge.target);
        }
        // Search roots in semantic order. The first successful root is the
        // smallest member of any concrete cycle; successor order is also fixed.
        for start in adjacency.keys() {
            let mut path = vec![*start];
            if cycle_from(*start, *start, &adjacency, &mut path) {
                let positions = path
                    .windows(2)
                    .filter_map(|pair| {
                        edges
                            .iter()
                            .find(|(edge, _)| {
                                edge.relation == relation
                                    && edge.source == pair[0]
                                    && edge.target == pair[1]
                            })
                            .map(|(_, index)| *index)
                    })
                    .collect();
                let names: Vec<_> = path.iter().map(|id| id.semantic_name()).collect();
                report(
                    diagnostics,
                    RuleId::M010,
                    path,
                    None,
                    DraftLocation::Cycle {
                        relation,
                        edges: positions,
                    },
                    format!("{} cycle: {}", relation.name(), names.join(" -> ")),
                );
                break;
            }
        }
    }
}

fn cycle_from(
    start: NodeId,
    current: NodeId,
    adjacency: &BTreeMap<NodeId, BTreeSet<NodeId>>,
    path: &mut Vec<NodeId>,
) -> bool {
    if let Some(successors) = adjacency.get(&current) {
        for next in successors {
            if *next == start {
                path.push(start);
                return true;
            }
            if !path.contains(next) {
                path.push(*next);
                if cycle_from(start, *next, adjacency, path) {
                    return true;
                }
                path.pop();
            }
        }
    }
    false
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionStage {
    GraphAdmission,
    RegistryAdmission,
    ReportConsumption,
}
