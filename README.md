# Field Theory of Overlap · `guide:ftoo:overview`

The Field Theory of Overlap is a formal theory of human ties — overlap, co-movement, caring, alignment, occupancy, belonging — developed as a paper that audits itself. Its design: the semantic content lives in one typed authority, the documents are generated from statement units, the notation is generated from a closed abstract syntax, and the mathematical claims are machine-audited with typed verdicts on a public certified-calculation engine.

## The design

The decision records in `docs/adr/` carry the full design with its reasons; `docs/glossary.md` carries the vocabulary. In brief, the repository is built so that:

- quantities, spaces, claims and their relations form a closed typed graph with four-axis statuses and a declared coverage boundary, making the audited slice a machine-checked fact (ADR 001, 002);
- theory objects are addressed by short human-facing tags bound to internal identifiers, failing the build when unresolvable (ADR 003);
- prose is authored as statement units carrying one text per language, with the LaTeX and Markdown documents both generated so they cannot drift (ADR 005, 006, 012);
- claims are audited through producers on the [Naturrhythmuszentrum engine](https://github.com/naturrhythmuszentrum/naturrhythmuszentrum) at a pinned public commit, with certified enclosures and typed undecided outcomes (ADR 007);
- every retained commit is signed and lands green through CI, each layer together with its validator (ADR 009), and anything published passes the publication audit (ADR 008, `docs/publication-policy.md`).

The open questions the repository records are listed in `generated/open-questions.md`, generated from `registry/open-questions.toml`; it holds the recorded questions, not every open question of the theory.

## Status

The design above is only partly built. The workspace crates are the following. `ftoo_manifest` holds the typed graph with its structural validator and numbered rule register, the coverage declaration, tag addressing, the closed notation syntax, statement units, the weld and the open-question records. `ftoo_codegen` generates and byte-checks the graph's projection `generated/manifest.json`, the notation and addressing files, the statement-unit documents under `generated/documents/`, and the open-question register, retains the audit report `generated/audit-report.json`, and typesets and byte-checks the paper pair. `ftoo_audit` audits the graph's bound claims on the engine at the commit pinned in `crates/ftoo_audit/Cargo.toml` and checks the retained audit evidence in `generated/audit-report.json`.

The graph holds only trial seed content, coefficient values of a Langevin expansion that exercise the graph, the audit and generation; no object or claim of the paper is in it yet. Its coverage declaration in `generated/manifest.json` is explicit-partial, and the audited slice it declares is those trial value claims. The statement-unit document `generated/documents/main/` is generated in Markdown and LaTeX from `units/`.

The paper and its explainer are not generated from statement units: they are authored LaTeX under `documents/paper/` with fixed data tables in `documents/paper/data/`, typeset reproducibly behind a byte gate (ADR 014). ADR 014 leaves open the generation of the data tables from the theory graph, the replacement of the typesetting layer file by file, and the entry of the paper's tags into the theory registry as the graph gains their objects. The open-question register records the data tables and the typed layer as machinery questions, whether the fixed data tables state what the graph states and which checks of the typed layer become numbered rules, beside recorded questions of the theory and of the explainer's claims. Page acceptance and publication of the paper are human steps outside the repository.

## Reading the paper

The paper is `generated/paper/paper.pdf` and its explainer is `generated/paper/explainer.pdf`; keep the two files in one directory so that the explainer's links into the paper resolve. `tools/paper/README.md` states the pinned support the build needs and the documented commands that regenerate both PDFs and check them byte for byte. The pair has no Markdown rendering (ADR 014).
