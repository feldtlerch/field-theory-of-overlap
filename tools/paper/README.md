# Complete English paper

The sources of the paper and its explainer are `documents/paper/`, listed in `documents/paper-build/sources.tsv`; the build records beside them are in `documents/paper-build/`. The reproducible pair is `generated/paper/paper.pdf` and `generated/paper/explainer.pdf`; keep them adjacent for companion links. The byte gate establishes reproduction only; it does not certify the theory or accept the page layout.

Typesetting runs on x86_64 Linux only, because the pinned engine is a static x86_64 Linux build. Provide the toolchain pinned by `rust-toolchain.toml`, the locked Cargo dependencies, the system PDF reader library `poppler-glib` (with GLib and GObject) for the independent text and destination inspection, and typesetting support prepared from `tools/assembly/native-lock.toml`:

```sh
python3 tools/assembly/prepare-native.py --support "$PWD/../paper-support"
export FTOO_ASSEMBLY_SUPPORT="$PWD/../paper-support"
```

That support holds the pinned engine and complete bundle; `documents/paper-build/runtime.toml` fixes their runtime settings. The `[version]` table of `documents/paper-build/templates.toml` is the one version setting: preparation substitutes its version number and draft name into both documents, so the paper's title page and version line and the explainer's version line and abstract read from the same value. Run these commands at the repository root. Both work directories must be absent; failed work is retained for diagnosis.

```sh
cargo run --locked -p ftoo_codegen --bin paper -- generate --work "$PWD/paper-build-work"
cargo run --locked -p ftoo_codegen --bin paper -- check --release --work "$PWD/paper-release-check"
```

`generate` overwrites `generated/paper/paper.pdf`, `generated/paper/explainer.pdf` and `generated/paper/inputs.json` with the fresh build; `check` changes no tracked file. Generation and checking share the same compiled driver. The fixed order is paper then explainer, with three explicit typesetting passes and one bibliography pass per invocation. The first equal consecutive complete auxiliary sets end the run; six pair cycles and 300 seconds per invocation are the limits. Timestamps, locale, work-area environment and runtime flags are fixed inputs. No cached verdict or extra confirmation build is used.

The receipt enumerates tracked byte inputs from the source list, Python input declarations and shared runtime lock. Every check and the support-required workspace test run the complete paper byte gate unconditionally. Hosted smoke checks skip support-required tests and never prepare typesetting support.

The paper's tag lookup has three statuses. The 329 spellings in `paper-tags.tsv` are paper-scoped: the paper's own manifest data (`documents/paper/data/ftoo_manifest.tex`) binds each to the label it defines, and none is admitted in either permanent registry; `reader-tags.toml` supplies three confirmed aliases that print paper spellings. The 87 admitted bindings in `reader-tags.toml` are documentary admissions recorded in `registry/documentary-tags.toml`, and `registry/reader-confirmation.md` records their exact decisions. `provisional-tags.toml` retains an empty candidate list; a future explicit proposal remains review-only. Missing keys refuse every build; `--release` also refuses any candidate, as a provisional fixture verifies. The `label-mints.tsv` inventory supplies documentary target evidence, and each of its rows names the source line that mints it; confirmation records remain independently minted Markdown records. No spelling enters a registry through generation.

M065 is the single paper release conjunction consumed by `paper check --release`: no provisional printed identity, acceptance matching all nineteen final passages, layout pins that cover exactly the line-addressed hooks with matching target hashes, and byte equality of both PDFs and the receipt. A failure names its part, for example `Paper release M065 refused at paper byte gate: Paper byte gate failed; generated/paper/paper.pdf: offset ...`, followed by lengths and expected/actual byte windows. An input mismatch instead names the stale receipt entry. This mechanical release check does not perform page acceptance or authorize publication, which remain separate human steps.

The admission context includes unit metadata and Markdown mints. The unconditional support-required gate checks that context independently of the byte-input receipt.

M063 checks binding integrity before typesetting, reusing documentary admission with the label-mint inventory. M064 checks shipped output: delayed writes discard measurement boxes, the final ledger resolves against auxiliaries and PDF destinations, and extracted page text and outline titles are checked for three-part leaks. A committed leak-control page (`tools/paper/fixtures/leak-control.tex`) prints a three-part key, and the support-required controls refuse unless the leak check finds it; a missing fixture refuses. The system PDF reader library supplies this independent inspection; it does not produce the delivered bytes. The work directory retains `identity-ledger.json`, `identity-proposals.json`, page text, outline titles and leak results from the stable cycle. The ledger participates in the auxiliary fixed point.

Every named destination must land on its labelled page. Any auxiliary/PDF page discrepancy refuses.

Every shipped occurrence must be visible on its recorded page. Any missing or surplus visible occurrence, including a clipped occurrence, refuses; no shipped page occurrence is exempt from the visible count.

`inputs.json` records only the tracked byte-input hashes. Checking requires its exact canonical bytes to match the fresh receipt, and reports the first differing entry. On a PDF mismatch the work area retains expected and actual PDFs, all auxiliary snapshots, byte windows between cycles and a comparison report naming changed inputs. Two PDF blobs add their actual changed bytes to history before compression; unchanged blobs are reused.
