# Native document coverage

[ADR 030](../adr/030-native-renderings-complete-graph-native-model.md) requires every paper and explainer passage to come from one native source for PDF, Markdown and HTML, and every rendered table to have a native body in each medium by release 0.6.0; a missing-content placeholder does not meet that boundary.

The identities below name measured file bytes, independent of the commit that contains them.

The 11 paper body files selected by `main.tex` and the explainer body are counted with each checked inclusion marker replaced by its generated fragment. Native means content from those inclusions; the remaining body TeX is authored. The paper's derived index and bibliography sections are counted as objects, but their entries are outside the body-word denominator.

## Classes

Each member is an occurrence of its class. Inline forms and objects may be nested, so class rows do not sum to a document block count. Native route names whether a checked route carries none, a subset, or all member forms; native counts measure inclusions in the body sources.

| Class | Paper native / members | Explainer native / members | All native / members | Native route | Basis |
|---|---:|---:|---:|---|---|
| inline mathematics | 1 / 1031 | 0 / 6 | 1 / 1037 | subset | Compared inherited symbols and observed HTML expressions cover selected contexts; `dec:notation:compared-inherited-native-support`, `rec:adr:042-html-mathematics`. |
| tag or handle references | 4 / 651 | 0 / 138 | 4 / 789 | subset | Checked paper tag runs and visible handles cover selected targets; `dec:documents:paper-tagged-paragraphs`, `rec:adr:048-reference-forms`. |
| paragraphs | 6 / 238 | 8 / 76 | 14 / 314 | subset | Tagged paragraph units and bounded explainer corpus slots; `dec:documents:paper-tagged-paragraphs`, `dec:documents:bounded-corpus-slot`. |
| headings | 0 / 149 | 2 / 14 | 2 / 163 | subset | Unnumbered explainer section headings in corpus slots; `dec:documents:bounded-corpus-slot`. |
| displayed equations | 0 / 141 | 1 / 1 | 1 / 142 | subset | The explainer's admitted equation unit covers one display form; M055 and `dec:documents:markdown-display-spacing`. |
| cross-references | 1 / 117 | 0 / 13 | 1 / 130 | subset | Checked see references and visible handles cover selected targets; `rec:adr:048-reference-forms`. |
| bibliography citations | 2 / 74 | 0 / 0 | 2 / 74 | subset | Verified parenthetical citations in native paragraphs and the economics box; `dec:documents:paper-tagged-paragraphs`, `dec:documents:native-econ`. |
| result box: definition | 0 / 42 | 0 / 0 | 0 / 42 | none | No native definition box is admitted. |
| tables | 0 / 32 | 0 / 3 | 0 / 35 | subset | A native display table exists outside the body inclusions; M055. |
| result box: econ | 1 / 34 | 0 / 0 | 1 / 34 | subset | The coda box's text and one parenthetical citation form; `dec:documents:native-econ`. |
| result box: theorem | 0 / 30 | 0 / 0 | 0 / 30 | none | No native theorem box is admitted. |
| refrains or quoted refrains | 5 / 17 | 3 / 12 | 8 / 29 | subset | Selected plain refrain units have a native route; M055. |
| result box: statement | 0 / 12 | 0 / 8 | 0 / 20 | none | No native statement box is admitted. |
| list items | 0 / 11 | 0 / 6 | 0 / 17 | none | No native list-item block is admitted. |
| result box: proposition | 0 / 17 | 0 / 0 | 0 / 17 | none | No native proposition box is admitted. |
| formal notes | 0 / 0 | 0 / 12 | 0 / 12 | none | No native formal-note block is admitted. |
| result box: corollary | 0 / 10 | 0 / 0 | 0 / 10 | none | No native corollary box is admitted. |
| result box: remark | 0 / 10 | 0 / 0 | 0 / 10 | none | No native remark box is admitted. |
| result box: insight | 0 / 9 | 0 / 0 | 0 / 9 | none | No native insight box is admitted. |
| lists | 0 / 4 | 0 / 2 | 0 / 6 | none | No native list block is admitted. |
| result box: lemma | 0 / 3 | 0 / 0 | 0 / 3 | none | No native lemma box is admitted. |
| result box: postulate | 0 / 3 | 0 / 0 | 0 / 3 | none | No native postulate box is admitted. |
| abstracts | 0 / 1 | 0 / 1 | 0 / 2 | none | No native abstract body is admitted. |
| contents | 0 / 1 | 0 / 1 | 0 / 2 | none | The TeX contents remains inherited. |
| bibliography section | 0 / 1 | 0 / 0 | 0 / 1 | none | The paper back matter is inherited; screen entries alone do not provide all three renderings. |
| granting notes | 0 / 1 | 0 / 0 | 0 / 1 | none | No native granting block is admitted. |
| result box: epist | 0 / 1 | 0 / 0 | 0 / 1 | none | No native epistemology box is admitted. |
| tag index section | 0 / 1 | 0 / 0 | 0 / 1 | none | The paper index remains a derived back-matter view. |
| figures | 0 / 0 | 0 / 0 | 0 / 0 | none | No native figure block is admitted. |
| footnotes | 0 / 0 | 0 / 0 | 0 / 0 | none | No native footnote block is admitted. |

## Paragraph forms

A paragraph is a lexical prose run outside boxes, notes, refrains, lists, tables, abstracts and displays. Forms overlap within a paragraph. A lexical one-symbol mathematics form is not a claim of mathematical simplicity. Title and setup text can still appear as structural artefacts. The corpus-slot run column describes only the bounded importer route, not other native paragraph units.

| Form | Paper members | Explainer members | All members | Corpus-slot run (`dec:documents:bounded-corpus-slot`) | Basis |
|---|---:|---:|---:|---|---|
| TeX quotes/dashes | 136 | 56 | 192 | yes | The importer decodes these source spellings into text. |
| tag/handle | 130 | 1 | 131 | not yet | The slot has no tag or handle run. |
| heading lead-in/context | 110 | 0 | 110 | not yet | The slot does not compose an arbitrary lead-in. |
| math: one symbol | 100 | 4 | 104 | not yet | The slot has no mathematics run. |
| emphasis | 68 | 35 | 103 | yes | The slot carries emph style runs. |
| math: compound | 89 | 1 | 90 | not yet | The slot has no mathematics run. |
| strong | 23 | 19 | 42 | yes | The slot carries textbf style runs. |
| other reference | 37 | 3 | 40 | not yet | The slot has no cross-reference run. |
| prose macro | 21 | 19 | 40 | not yet | The slot has no prose-expanding macro run. |
| display adjacency | 39 | 0 | 39 | not yet | The slot does not compose an adjacent display. |
| plain prose only | 4 | 12 | 16 | yes | The slot carries text runs. |
| layout hook | 13 | 1 | 14 | not yet | The slot does not carry paragraph layout hooks. |
| en-space | 0 | 6 | 6 | yes | The slot carries the named en-space intent. |
| other spacing | 5 | 1 | 6 | not yet | The slot admits only named en-space intent. |
| citation | 4 | 0 | 4 | not yet | Citation is not a corpus-slot run. |
| other command | 2 | 1 | 3 | not yet | The closed importer refuses unclassified commands. |
| structural artefact | 2 | 1 | 3 | not yet | A counted run may be structural text rather than a prose paragraph. |
| code mention | 2 | 0 | 2 | not yet | Opaque code is not a corpus-slot run. |
| other style | 1 | 1 | 2 | not yet | The slot admits emph and textbf styles only. |
| title typography | 1 | 1 | 2 | not yet | The slot has no title-typography run. |
| escaped punctuation | 0 | 0 | 0 | yes | The importer decodes these escaped source characters. |
| external link | 0 | 0 | 0 | not yet | The slot has no link run. |
| footnote | 0 | 0 | 0 | not yet | The slot has no footnote run. |
| list adjacency | 0 | 0 | 0 | not yet | The slot does not compose an adjacent list. |
| note/refrain adjacency | 0 | 0 | 0 | not yet | The slot does not compose an adjacent note or refrain. |
| note/refrain wrapper | 0 | 0 | 0 | not yet | The slot has no note or refrain child. |

## Explainer capability unlocks

A capability unlock counts an authored explainer paragraph whose only missing detected capability is that row. The two mathematics forms share one mathematics-run capability. This lexical screen does not prove importer admission or transfer of the surrounding source.

| Missing capability | Authored paragraphs opened |
|---|---:|
| prose macro | 14 |
| other reference | 2 |
| layout hook | 1 |

| Document | Paragraph members | Native | Using only admitted forms | Authored using only admitted forms |
|---|---:|---:|---:|---:|
| paper | 238 | 6 | 12 | 12 |
| explainer | 76 | 8 | 51 | 44 |
| all | 314 | 14 | 63 | 56 |

Of the 44 authored explainer paragraphs using only admitted forms, 12 lie wholly within admissible intervals from the [importer's dry run](../adr/053-whole-corpus-import.md). The importer reports slot intervals for the explainer only.

## Importer blocks and refusals

The importer owns block splitting and diagnostic refusal counts. Its counts below differ from the lexical paragraph and word measures above; neither measure substitutes for the other.

| Kind | Fully decoded | Refused |
|---|---:|---:|
| abstract | 0 | 2 |
| display | 0 | 23 |
| economics-box | 0 | 33 |
| epistemology-box | 0 | 1 |
| existing-native-inclusion | 19 | 0 |
| heading | 156 | 2 |
| list | 0 | 2 |
| paragraph | 112 | 232 |
| refusal | 0 | 12 |
| statement-box | 0 | 20 |
| table | 0 | 33 |
| theorem-container | 11 | 113 |

| First refusal kind | Outer blocks |
|---|---:|
| `unresolved mathematics` | 119 |
| `unsupported result child` | 47 |
| `unsupported environment econ` | 33 |
| `unsupported environment table` | 33 |
| `unsupported environment theorem` | 30 |
| `unsupported environment statement` | 20 |
| `unsupported environment equation` | 19 |
| `unsupported environment proposition` | 17 |
| `unsupported command \\gloss` | 16 |
| `unsupported command \\formalfoot` | 12 |
| `unsupported command \\refrain` | 12 |
| `comment in content` | 10 |
| `unsupported command \\zcref` | 10 |
| `unsupported environment corollary` | 10 |
| `unsupported command \\appblock` | 9 |
| `unsupported command \\paperrefrain` | 9 |
| `unsupported environment center` | 6 |
| `unsupported command \\gradeword` | 5 |
| `unsupported command \\see` | 4 |
| `unsupported environment align` | 4 |
| `unsupported environment serieslist` | 4 |
| `unsupported command \\addcontentsline` | 3 |
| `unsupported command \\auditnote` | 3 |
| `unsupported environment lemma` | 3 |
| `unsupported environment postulate` | 3 |
| `unsupported result header` | 3 |
| `unsupported command \\gateway` | 2 |
| `unsupported command \\hypersetup` | 2 |
| `unsupported command \\registername` | 2 |
| `unsupported command \\texorpdfstring` | 2 |
| `unsupported environment abstract` | 2 |
| `unsupported environment itemize` | 2 |
| `unsupported environment xltabular` | 2 |
| `unsupported command \\GenerateBranchLedger` | 1 |
| `unsupported command \\GenerateConcordanceE` | 1 |
| `unsupported command \\GenerateEquivalences` | 1 |
| `unsupported command \\GenerateEvidence` | 1 |
| `unsupported command \\GenerateLedger` | 1 |
| `unsupported command \\GenerateRulings` | 1 |
| `unsupported command \\PaperNeedLines` | 1 |
| `unsupported command \\PaperParagraph` | 1 |
| `unsupported command \\hfill` | 1 |
| `unsupported command \\papervirtue` | 1 |
| `unsupported command \\setcounter` | 1 |
| `unsupported command \\texttt` | 1 |
| `unsupported command \\verb` | 1 |
| `unsupported command \\virtue` | 1 |
| `unsupported environment epist` | 1 |

| Explainer admissible interval | Stopping condition |
|---|---|
| 212–216 | `documents/paper/explainer/explainer.tex:217 StatementBox Unresolved` |
| 630–650 | `documents/paper/explainer/explainer.tex:651 Paragraph Unresolved` |
| 677–681 | `documents/paper/explainer/explainer.tex:682 StatementBox Unresolved` |
| 752–762 | `documents/paper/explainer/explainer.tex:763 Paragraph Unresolved` |
| 816–847 | `documents/paper/explainer/explainer.tex:848 Paragraph Unresolved` |
| 856–886 | `documents/paper/explainer/explainer.tex:887 ExistingNativeInclusion Decoded` |

## Coverage by reader words

| Document | Native words / all | Word share |
|---|---:|---:|
| paper | 647 / 28919 | 2.24% |
| explainer | 1006 / 8601 | 11.70% |
| all | 1653 / 37520 | 4.41% |

## Per-file detail

Word tokens count literal headings, captions, table cells and prose; they exclude comments, blank lines, preambles, command names and keys, math notation, reference and citation labels, and macro expansions. Multi-line mathematics and TeX-produced prose can escape this lexical count.

| Document | Source | Native words / all | Word share | Source blob |
|---|---|---:|---:|---|
| paper | [`documents/paper/parts/front.tex`](../../documents/paper/parts/front.tex) | 57 / 2163 | 2.64% | `72978326c5c69231947c0766c9c1639f1dab0db3` |
| paper | [`documents/paper/parts/partA.tex`](../../documents/paper/parts/partA.tex) | 40 / 3944 | 1.01% | `26d056721554dd62b923ba311beb18150c77bd0a` |
| paper | [`documents/paper/parts/partB.tex`](../../documents/paper/parts/partB.tex) | 0 / 6180 | 0.00% | `416f396158d7209429a5df48e37f5a99949cbf1d` |
| paper | [`documents/paper/parts/gateways.tex`](../../documents/paper/parts/gateways.tex) | 0 / 863 | 0.00% | `438ebdb6c984ab4d11308d1eb8cb18d20798cf5b` |
| paper | [`documents/paper/parts/partC.tex`](../../documents/paper/parts/partC.tex) | 0 / 2569 | 0.00% | `61498045f71dd7ed81f1a27314cd685596169252` |
| paper | [`documents/paper/parts/partD.tex`](../../documents/paper/parts/partD.tex) | 6 / 2690 | 0.22% | `0774854886fc1d98d0287911ab3646d1a4b5d936` |
| paper | [`documents/paper/parts/partE.tex`](../../documents/paper/parts/partE.tex) | 0 / 1103 | 0.00% | `56c378ff90abc4e56c601b06099cf020d1655115` |
| paper | [`documents/paper/parts/partF.tex`](../../documents/paper/parts/partF.tex) | 45 / 2570 | 1.75% | `c735755ba9dec5f857e64847a36d080fb23248ce` |
| paper | [`documents/paper/parts/partG.tex`](../../documents/paper/parts/partG.tex) | 0 / 1369 | 0.00% | `affe4e9abf3c661d94e9efbfcfab29c4e015c844` |
| paper | [`documents/paper/parts/coda.tex`](../../documents/paper/parts/coda.tex) | 463 / 992 | 46.67% | `f32fad8baa9a806cb745c984509a53ebdad356f0` |
| paper | [`documents/paper/parts/appendices.tex`](../../documents/paper/parts/appendices.tex) | 36 / 4476 | 0.80% | `6f5d0d158aa6316f511be9353162e911739fc3f5` |
| paper | Total | 647 / 28919 | 2.24% | — |
| explainer | [`documents/paper/explainer/explainer.tex`](../../documents/paper/explainer/explainer.tex) | 1006 / 8601 | 11.70% | `3bf2f062080705491a258c1c00a2b28d7de24fed` |
| explainer | Total | 1006 / 8601 | 11.70% | — |
| all | Total | 1653 / 37520 | 4.41% | — |

## Limits and regeneration

This source inventory cannot determine whether native text is still compared with retained wording, what a TeX macro expands into as prose, the printed words of references and citations, or whether a rendered link or object is correct. Mathematical annotations and derived index and bibliography entry text are not body words here. The form classifier reads lexical spellings and nearby boundaries; it cannot prove importer admission, source correspondence or complete rendering. It is not the PDF-extracted word comparison.

A later inventory can take its block and form boundaries directly from the importer's residue and leave only word share to the lexical reader. That would remove the present duplicate boundary interpretation while retaining the source-word measure the importer does not provide.

Input identities are Git blob IDs of the measured bytes. The importer source-list digest is SHA-256; its source digests in the snapshot bind the diagnostic measurement to all twelve body sources.

| Input | Blob ID |
|---|---|
| `documents/paper/main.tex` | `06bf235acb58b61df74058a8411e6b64a84d2bed` |
| `documents/paper-build/sources.tsv` | `03eb4406b96f3f730b7b4113b9306983b2e88ad3` |
| `generated/documents/inclusions.tsv` | `16d838b66ce71b026ecec890a5098b5083615924` |
| `tools/paper/native_coverage_capabilities.tsv` | `5e29398a51a75639d170462968df7539ae897c20` |
| `tools/paper/native_coverage_slot_census.json` | `2293ed157eb0767dbf6046c5519b349aab9eac70` |
| `generated/documents/explainer-pilot/eq-explainer-overlap-flow.tex` | `b85e6aad8526aac08e02ea0bcf591bedf9fc060e` |
| `generated/documents/explainer-pilot/sec-coda-record-limit.tex` | `a65badd079a35732942e07e0142456f85c0b96f3` |
| `generated/documents/explainer-pilot/sec-explainer-accountability.tex` | `288faebf96c340e284e83cb1a248ca936cb0024f` |
| `generated/documents/explainer-pilot/sec-explainer-eight-words.tex` | `209d37c06c43da9a06c305d41b9bca1bf288e830` |
| `generated/documents/explainer-pilot/sec-explainer-plain.tex` | `dbc811c11427ce161a883be934582a4ed7473b09` |
| `generated/documents/explainer-pilot/sec-say-direction.tex` | `3c7f5590a578c5dcced2cebf2f856faff2f677df` |
| `generated/documents/explainer-pilot/sec-say-enemy-to-none.tex` | `24799d78631c1e1fbb6bd57e83b5fa9b49d78995` |
| `generated/documents/paper-pilot/sec-boundary-ratio-rate.tex` | `53b2cee7819403c753f604a395517dd07dc9266b` |
| `generated/documents/paper-pilot/sec-coda-audit-bottoms.tex` | `54dd9cce20c26390e517c3a16e8227e008988439` |
| `generated/documents/paper-pilot/sec-coda-eight-words.tex` | `db01f1b2c4f4f0d5a03d3355fd11dca3877aecb0` |
| `generated/documents/paper-pilot/sec-coda-flourishing-pole.tex` | `ff6de3c59c37ec44d9f485e3aa211049f539e90c` |
| `generated/documents/paper-pilot/sec-coda-record-limit.tex` | `4e692f961a84b1c7e31b17aa300249dd742057f6` |
| `generated/documents/paper-pilot/sec-coda-whole-field.tex` | `9a1406a274eef4ae74b9af51d7880160bde5bc7c` |
| `generated/documents/paper-pilot/sec-log-andreoni-joint.tex` | `503cc3699a946fc6ea4b2048d535e648c50a29ed` |
| `generated/documents/paper-pilot/sec-reading-shadow.tex` | `4d24564f24d4c7860a8fd3841246a07a116d0c43` |
| `generated/documents/paper-pilot/sec-say-direction.tex` | `21a05ec3f67d78af620773dde80c8cce27d783e8` |
| `generated/documents/paper-pilot/sec-say-enemy-to-none.tex` | `7ae2609a06b0638666ac3c42b334e4cc0ac2e778` |
| `generated/documents/paper-pilot/sec-say-harm-enlargement.tex` | `d176ea92cb1354422be09006170e568edeaac5a2` |
| `generated/documents/paper-pilot/sec-say-wonder-beauty.tex` | `8c9970a011b7e79099ee66857e9fb610893879e3` |

Importer source-list digest: `e3fa37f59b91c9d640737b309e7dcbd72e532fd29d998f6841b0ef117ea111fd`.

Regenerate this page with `python3 -B tools/paper/native_coverage.py`. The maintained capability judgments are in `tools/paper/native_coverage_capabilities.tsv`. After a body-source edit, run `paper import-dry-run --repository ROOT --output-dir DIR`, then `python3 -B tools/paper/native_coverage.py --import-residue DIR/residue.json`. The snapshot records the importer's measured sources and intervals; the page is then regenerated and checked with the other generated documents.
