# Field Theory of Overlap · `guide:ftoo:overview`

The Field Theory of Overlap is a formal theory of human ties — overlap, co-movement, caring, alignment, occupancy, belonging — developed as a paper that audits itself. Its design: the semantic content lives in one typed authority, the documents are generated from statement units, the notation is generated from a closed abstract syntax, and the mathematical claims are machine-audited with typed verdicts on a public certified-calculation engine.

## The design

The decision records in `docs/adr/` carry the full design with its reasons; `docs/glossary.md` carries the vocabulary. In brief, the repository is built so that:

- quantities, spaces, claims and their relations form a closed typed graph with four-axis statuses and a declared coverage boundary, making the audited slice a machine-checked fact (ADR 001, 002);
- theory objects are addressed by short human-facing tags bound to internal identifiers, failing the build when unresolvable (ADR 003);
- prose is authored as statement units carrying one text per language, with the LaTeX and Markdown documents both generated so they cannot drift (ADR 005, 006, 012);
- claims are audited through producers on the [Naturrhythmuszentrum engine](https://github.com/naturrhythmuszentrum/naturrhythmuszentrum) at a pinned public commit, with certified enclosures and typed undecided outcomes (ADR 007);
- every retained commit is signed and lands green through CI, each layer together with its validator (ADR 009), and anything published passes the publication audit (ADR 008, `docs/publication-policy.md`).

## Status

Pre-release skeleton. The decision records are complete; the crates are empty shells awaiting their first reviewed bites; the engine pin lands with the audit crate's first bite, and the capability requests it needs will be linked here as public issues once filed. The coverage declaration will state at every point how far the audited slice reaches.

## Reading the paper

The generated Markdown rendering will be the no-toolchain path, living beside the LaTeX documents once the first parts land; both derive from the same units.
