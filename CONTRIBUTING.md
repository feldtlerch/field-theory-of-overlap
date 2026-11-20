# Contributing · `guide:ftoo:contributing`

## Read first

Read the [README](README.md) for what is built, the [ADR register](docs/adr/README.md) for decisions and their governing clauses, and the [glossary](docs/glossary.md) for vocabulary. The **pair** means the paper and explainer PDFs; their **receipt**, `generated/paper/inputs.json`, records the tracked byte-input hashes.

## Find the source

|Location|Contents|
|---|---|
|`crates/ftoo_manifest/`|Typed authority, identities, validation register, tags, notation and unit admission.|
|`crates/ftoo_audit/`|Claim producers, pinned engine integration and retained audit evidence.|
|`crates/ftoo_codegen/`|Shared generation and byte checking, including the `paper` binary.|
|`documents/paper/`|Authored LaTeX and fixed data tables for parts not yet generated from units; replaced file by file under ADRs 014 and 016.|
|`documents/paper-build/`|Records checking still-authored sources: source list, settings, tag bindings, mint evidence and accepted passages; each ends with the source it checks under ADR 016.|
|`units/`|Authored statement texts, per-target display bodies, declarations and skeletons.|
|`registry/`|Documentary tags, confirmation evidence and open questions; theory tags live in the manifest crate.|
|`generated/`|Committed projections, notation, addressing, audit evidence, question register, documents, fragments and PDFs.|
|`tools/`|Checks, support declarations, generation adapters and inspection tools.|
|`docs/`|ADRs, design notes, vocabulary, engineering lessons and publication policy.|
|`licenses/`|Third-party licence texts and font notices.|
|`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`|Workspace, locked dependencies and pinned Rust toolchain.|
|`meson.build`, `meson.options`, `meson/`|Build graph, configuration and native-file template.|
|`.github/`|Hosted checks.|

Change authored sources and regenerate; never repair a rendering by hand. Document completeness does not enlarge audit coverage under [ADR 016](docs/adr/016-admission-and-native-documents.md) (`dec:documents:admission-and-native-renderings`).

## Run the checks

Reproducibility is exact: the same declared inputs produce the same bytes, and every committed generated artifact equals its generator's output; generation and checking share one code path. This is decided in [ADR 006](docs/adr/006-byte-gated-generation.md) (`dec:generation:byte-gated`), rests on [ADR 013's locked inputs](docs/adr/013-workspace-norms.md) (`arg:workspace:reasons`), and is applied to the PDF pair by ADRs 014 and 019.

Use the pinned toolchain and locked dependencies. Do not raise limits, defaults or pins to pass checks under ADR 013. Typesetting runs on x86_64 Linux only; [tools/paper/README.md](tools/paper/README.md) specifies its support and PDF inspection dependencies. Export these variables as absolute paths, following [ADR 020](docs/adr/020-meson-ninja-build.md):

|Variable|What it is|How to obtain it|
|---|---|---|
|`SRC`|Source checkout|Use its absolute root.|
|`BUILD`, `CARGO_TARGET_DIR`|Stable per-checkout build and Rust output directories|Choose locations outside `SRC`.|
|`NATIVE`|Executable configuration|Copy `meson/native.ini.in`; replace each placeholder with an absolute executable path.|
|`NINJA`|Pinned Ninja executable|Use the executable named in `NATIVE`.|
|`SUPPORT`|Prepared native support|Follow the preparation command in `tools/paper/README.md`.|
|`NOTATION_SUPPORT`|Prepared notation support|Pins and checks are in `tools/notation/` and the codegen notation module; its preparation is a gap below.|
|`CARGO_CACHE`|Preseeded offline Cargo home|Must contain locked registry and Git sources; seeding is a gap below.|

PATH must resolve `meson`, `ninja`, `python3`, `cargo`, `rustc` and `node` to the configured executables. Setup enforces the Meson and Ninja pins. At the repository root, check the cache, then configure a new build directory:

```sh
CARGO_HOME="$CARGO_CACHE" cargo fetch --locked --offline --manifest-path "$SRC/Cargo.toml"
meson setup "$BUILD" "$SRC" --backend=ninja --wrap-mode=nodownload --native-file "$NATIVE" --prefix=/ -Dsupport="$SUPPORT" -Dnotation_support="$NOTATION_SUPPORT" -Dcargo_target_dir="$CARGO_TARGET_DIR" -Dcargo_home="$CARGO_CACHE"
```

Run ADR 020's ordinary round with formatting, Clippy and labels equivalent to the [hosted workflow's](.github/workflows/ci.yml) checks. Stop if a command fails:

```sh
cargo fmt --all -- --check
CARGO_HOME="$CARGO_CACHE" cargo clippy --workspace --all-targets --locked --offline -- -D warnings
python3 -I -B tools/check_labels.py --self-test
python3 -I -B tools/check_labels.py
"$NINJA" -C "$BUILD" -j 1 meson-test-prereq
meson test -C "$BUILD" --no-rebuild --suite unit --suite product --suite regression --print-errorlogs
```

|Check in [meson.build](meson.build)|What it checks|
|---|---|
|`unit`|Offline workspace tests, `--no-fail-fast`, skipping `support_required`: admission, labels, audit and support-independent artifact checks.|
|`regression`|Supported native and notation tests, excluding `support_required_paper_gate`.|
|`documents-verified`, then `pair`|Committed unit-derived artifacts before production of the PDFs and receipt.|
|`product`|`byte-gate` compares PDFs and receipt; `live-release` checks M065 and documents; `evidence-T40` consumes the separate identity fixture.|
|`graph`|G1 checks the content-record cutoff and G2 publication refusal and retention behaviour, both on a nested Meson fixture.|

For the separate graph suite, run `"$NINJA" -C "$BUILD" -j 1 meson-test-prereq`, then `meson test -C "$BUILD" --no-rebuild --suite graph --print-errorlogs`. Hosted CI runs formatting, denied-warning Clippy, support-independent tests and labels with network-isolated test processes; it does not replace the full gate.

For intentional document changes, run `meson compile -C "$BUILD" documents-generate`, then `"$NINJA" -C "$BUILD" -j 1 meson-test-prereq`. The generation target writes `generated/documents/`, `generated/notation.tex` and `generated/addressing.tex`. Install the built PDFs and receipt with `meson install -C "$BUILD" --no-rebuild --tags doc --destdir "$SRC"`, inspect the diffs and rerun checks. [ADR 015](docs/adr/015-open-questions.md) gives the separate question-register regeneration command; [the paper instructions](tools/paper/README.md) give the forced Meson-free route.

Ordinary pair reuse still compares actual built and committed bytes. A release or engine, toolchain or graph-boundary change requires two independent builds under [ADR 019](docs/adr/019-reused-paper-pair.md) (`dec:generation:reused-paper-pair`). Set `RELEASE_A` and `RELEASE_B` to empty absolute build directories and `RELEASE_A_CARGO` and `RELEASE_B_CARGO` to distinct absolute target directories, all outside `SRC`. Run [ADR 020's six-command release sequence](docs/adr/020-meson-ninja-build.md); both product suites compare their own production against committed bytes.

### Documentation-only changes

[ADR 018](docs/adr/018-docs-only-gate.md) permits inheritance of unchanged document bytes (`dec:docs-gate:equivalence`). Use its separate clean base and candidate checkout procedure, where `BASE` is the reviewed, green base commit, an ancestor of candidate HEAD. Export `CARGO_TARGET_DIR` outside both checkouts and provide locked dependencies offline:

```sh
python3 -I -B BASE_CHECKOUT/tools/docs_gate.py BASE --repository CANDIDATE_CHECKOUT
```

Eligible records are root Markdown, `LICENSE-CODE` and `LICENSE-DOCS`, Markdown under `docs/`, licence texts under `licenses/` and tooling READMEs, only outside protected inputs; every crate, document source, unit, registry entry and generated artifact is protected, as are the native lock, preparation script, gate code and tests, and unknown paths are refused. Follow ADR 018 for checkout construction, cleanliness and the exact checks. Exit zero is PASS for the named candidate; two is refusal requiring the full gate; `--classify-only` exits three for eligibility without PASS. Every retained intermediate commit needs its own verdict.

## Shape a change

What a retained commit needs under [ADR 009](docs/adr/009-process.md) (`dec:process:signed-bites`):

- Reviewable scope, a signature and green CI; [SIGNING.md](SIGNING.md) identifies the key and verification command.
- Each layer together with its checking validator or consumer, never ahead of it.
- A message stating what holds and why, timelessly; checks report wall time.

History uses `type(scope): summary` with an engineering-reason body and no attribution trailers; examples include `feat(documents)`, `fix(codegen)`, `docs(adr)`, `build(paper)`, `test(audit)`, `perf(support)` and `chore(deps)`. Inspect it with `git log --format=full`; these observations are not a written message policy. The history has no merge commits. `main` is the hosted workflow's push branch; pull requests are also checked.

Tooling dependencies enter only at public pinned versions ([ADR 008](docs/adr/008-publication.md)). Toolchain and engine-pin moves are reviewed changes (ADRs 013 and 007). A capability another project can state without naming this theory belongs in the engine's public issues; theory-specific work stays here under [ADR 007](docs/adr/007-audit-on-the-engine.md) (`dec:audit:engine-of-record`).

Everything published, history included, is audited for private references under [the publication policy](docs/publication-policy.md) (`rec:policy:publication-audit`). Keep credentials, private host names and addresses, account names, personal absolute paths, personal email addresses and build-log residue out of files and messages, subject to that policy's published-identity and third-party-notice exceptions. Release requires zero critical and zero major findings (`rule:policy:report-gate`).

## Record decisions and invariants

The ADR register records accepted status and the clauses later records govern. Propose an ADR in four blocks: **Decision (Statement)**, **Reasons (Why)**, **Declined (Alternatives)** and **Consequences (Accepted)**. It lands accepted after review. A later ADR changing an earlier one names, and usually quotes, the clause it changes; the register's State column records which record governs (`conv:adr:form`).

[The validation register](crates/ftoo_manifest/src/validation.rs) records each permanent number, statement, scope, state and deferred reason. `Structural` means admitted Rust types make violation impossible; `Enforced` means checked; `DeferredM2`, `DeferredExclusiveRoutes` and `DeferredPaperUnits` retain visible debt. New rules carry those fields, and scope changes require review under [ADR 010](docs/adr/010-validation-rules.md) (`dec:graph:validation-rules`).

The nearest written statements to a general invariant policy are ADR 009's layer-with-validator rule, ADR 010's requirement to register unenforced rules as visible debt with scope stated outside code paths, and ADRs 016–017's distinction between recorded obligations and claims of enforcement. The universal policy remains a gap below.

- Closed typed graph — [ADR 001](docs/adr/001-closed-typed-graph.md), qualified by ADR 026 (`dec:graph:closed-authority`).
- Byte-gated generation — ADR 006, qualified by ADRs 018–019 (`dec:generation:byte-gated`).
- Permanent bindings and reader handles — [ADR 003](docs/adr/003-tags-bound-to-labels.md), qualified by ADRs 016 and 017 (`dec:tags:bound-to-labels`).
- Readers never see labels — [ADR 017](docs/adr/017-screen-rendering-contracts.md) (`dec:documents:screen-rendering-contracts`).
- Native-source obligations — ADR 016 (`dec:documents:admission-and-native-renderings`).
- Reproducible paired PDFs — [ADR 014](docs/adr/014-paper-pair.md) and ADR 019 (`dec:paper:authored-latex`, `dec:generation:reused-paper-pair`).
- Public inputs and publication audit — ADR 008 (`dec:publication:fresh-history`).

## Labels and tags

A label has the form `kind:area:slug`, using lowercase letters, digits and hyphens. Mint it once at the statement it names, using the middle-dot and single-backtick form in this guide's heading. Cite it in parentheses with single backticks, as in (`conv:adr:labels`). Any such label inside parentheses counts as a citation to both checkers: keep placeholders and mentions outside parentheses. Choose the kind for the statement's role; `open` belongs only to the question store (M068). The kinds `read` and `audit` are kept for reading declarations and derived audit identities; they cannot be minted as theory, Markdown or unit labels, used as paper label-mint keys or tag targets, or admitted as documentary targets (M073).

A unit's id follows the three-component grammar. ADR 016 decides that a separate, closed documentary-target field in unit metadata may mint admitted targets outside that grammar; [the unit loader](crates/ftoo_codegen/src/units/toml.rs) does not implement it yet, and until it does such targets are evidenced only by the paper's label-mint inventory (M060). That inventory records source mints while authored paper sources remain; unit prose does not mint labels.

A reader handle is called a **tag** in the decision records, glossary and registry files. Reader citations need a unique live handle (ADR 003). ADR 016 decides that address availability will not depend on audit coverage and that handles are required only where readers address objects; rules M035 and M036 still implement the earlier out-of-slice store and full theory coverage until revised. Documentary bindings are immutable, retirement is final and label-shaped documentary handles are refused (M060).

Reader text refuses literal labels (M055). Under ADR 017, `en.md` contains no internal labels, even in raw source; `en.labels.md` retains recovery evidence. The Python commands above, run at the repository root, check Markdown harvesting, mint uniqueness and citation resolution. [Manifest labels](crates/ftoo_manifest/src/labels.rs) and [codegen integration](crates/ftoo_codegen/src/labels.rs), exercised by workspace tests, check across Rust identities, Markdown and unit metadata; the Python check alone cannot prove that wider uniqueness.

## Review renderings and licences

Changed pages and screens need recorded independent review against a prepared acceptance sheet under ADR 017. Follow [tools/paper/README.md](tools/paper/README.md) for baseline/candidate hashes, changed and adjoining page images, rendered screens, raw comparisons, destinations, links and the other evidence it lists. Record reviewed artifact coverage, findings and acceptance or revision. The author cannot accept the change; changed bound passages need renewal under ADR 024. A change to the theory requires approval by the project's owner; every other change, including a meaning-preserving passage renewal, is accepted through independent review by a reviewer other than the change's author, with its evidence recorded. When a claim of the theory is qualified or changed, the change is given to the project's owner for review before it lands. M065 conjoins the mechanical release checks its register entry lists; it neither supplies visual acceptance nor authorizes publication.

[LICENSING.md](LICENSING.md) assigns MIT-0 to crates and tooling and CC0 to the paper, explainers and decision records. Authored paper typesetting files are document sources under `LICENSE-DOCS` (ADR 014). Fonts retain their own licences and notices; follow the recorded embedding conditions. Markdown carries no fonts.

## Not yet written down

- Branch workflow, merge/rebase or linear-history policy, and whether `main` guarantees release readiness.
- Required commit grammar, body fields, permitted types/scopes and an attribution-trailer prohibition.
- How outside contributors obtain project signing and how their authorship is recorded.
- A universal rule that every invariant is labelled, recorded and argued before assertion, or that longstanding unrecorded properties are not repository invariants.
- Complete notation-support preparation and offline Cargo-cache seeding recipes.
- When the separate graph suite is mandatory.
- A definition of “the full gate”; the nearest written set is ADR 020's ordinary round with formatting, Clippy and label checks.
- Documented regeneration commands for `generated/manifest.json` and `generated/audit-report.json`; document generation does not regenerate them, and the question register has its own ADR 015 command.
- Where page/screen review records are kept and how outside contributors obtain independent reviewers.
- Which practical accessibility checks to perform and record.
- A single publication-audit command.
- Licence coverage for units, registries, design notes and the glossary, and terms for accepting contributions, including any DCO or CLA.
- The complete label-kind taxonomy referred to by the ADR conventions.
