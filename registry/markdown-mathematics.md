# Markdown mathematics observations · `reg:documents:markdown-mathematics-observations`

The retained 2026-11-18 candidate pages give each exact body and its surrounding Markdown or HTML context; each M entry and A/B variant in the observation record gives its appearance and the received engine input where captured. Received bytes were measured for M02, M03, M05, M07, M12A, M22B, M25, M31, M42A and M47A; other transport conclusions are inferred from appearance. The signed-out forge view showed public commit 3e95972 at experiments/markdown-view/math/ on 2026-11-18 in Chromium 156 on Linux. Its mathematics engine was unidentified, no mathematical web font loaded, and no installed-font inventory or persistent screenshot or DOM extract was captured. These limits apply to the 2026-11-18 rows below.

The cases are indexed per candidate page; each page holds its variants and exact source context.

| Cases | Candidate page or source | Observation | Admission |
| --- | --- | --- | --- |
| M01–M08 | [delimiters](markdown-mathematics/2026-11-18/candidates/01-delimiters.md) | [record](markdown-mathematics/2026-11-18/observation.txt) | M02's protected inline spelling; M04's separate-line display spelling; other spellings are evidence only. |
| M09A/B–M15A/B | [punctuation](markdown-mathematics/2026-11-18/candidates/02-punctuation.md) | [record](markdown-mathematics/2026-11-18/observation.txt) | Protected inline in ordinary prose; delimiter, table, HTML, emphasis and citation-boundary characters in the body are refused. |
| M16A/B–M24A/B | [contexts](markdown-mathematics/2026-11-18/candidates/03-contexts.md) | [record](markdown-mathematics/2026-11-18/observation.txt) | Protected inline in prose, a table body cell, a simple top-level list item, a level-four heading, or beside a comment with a separating space; HTML and direct comment adjacency are refused. |
| M25–M30 | [fenced displays](markdown-mathematics/2026-11-18/candidates/04-display-fences.md) | [record](markdown-mathematics/2026-11-18/observation.txt) | Evidence only; fences are not an admitted display spelling. |
| M31–M36 | [dollar displays](markdown-mathematics/2026-11-18/candidates/05-display-dollars.md) | [record](markdown-mathematics/2026-11-18/observation.txt) | Separate-line double-dollar top-level blocks with blank lines around them; M34 tags and M35 numbering are evidence only. |
| M37A/B–M42A/B | [expanded notation](markdown-mathematics/2026-11-18/candidates/06-expanded-notation.md) | [record](markdown-mathematics/2026-11-18/observation.txt) | Transport evidence only; primitive admission remains with checked notation. |
| M43–M47A/B | [paper constructs](markdown-mathematics/2026-11-18/candidates/07-paper-constructs.md) | [record](markdown-mathematics/2026-11-18/observation.txt) | Evidence only; unresolved fonts, boxes, operators and prime appearance do not authorize replacement spellings. |
| M48A/B–M51A/B | [failure controls](markdown-mathematics/2026-11-18/candidates/08-failure-controls.md) | [record](markdown-mathematics/2026-11-18/observation.txt) | Evidence only; rejection of an unknown macro or package command does not admit arbitrary commands. |
| Display spacing, 2026-11-24 | A fourteen-case probe on a test branch, not retained here; the generated explainer reader files are hash-identified in the record | [record](markdown-mathematics/2026-11-24/flow-spacing-observation.md) | The checked dimensioned thick-space spelling in the separate-line double-dollar display under ADR 051; the fenced result is evidence only. |
| Inherited single-symbol glyphs | `math-catalogue-symbols.md`, SHA-256 `7852b40d40cbc34965af0d2d276a0acbcc49be6fd98751cb2fe9f10bc98d342e`; companion PDF SHA-256 `4beb029600c48a94fc9e120b5d0aad9d86d7e842b533868232de10123df6b8d0` | Forge look pending; the companion PDF was typeset from the pinned mathematics bundle and its text extracted. | No additional glyph spelling is admitted by this row before the forge look is recorded. |

The [observation sheet](markdown-mathematics/2026-11-18/observation-sheet.md) is the recording protocol with each case's expected appearance; results are in the [observation record](markdown-mathematics/2026-11-18/observation.txt). The observation record and sheet are retained with the observing setup described in neutral terms and their measurements unchanged. Missing screenshots, retained DOM and font inventory cannot be inferred from the textual observations. New observations add a dated evidence set and a row here, leaving this record's earlier observations intact.

| Retained file | SHA256 |
| --- | --- |
| `candidates/01-delimiters.md` | `40f00866cef5cb0979b25638d3b2c690e59ab54d6bc9d0203501f0748bde26a0` |
| `candidates/02-punctuation.md` | `ce0e75b1042208ae8575c37b9906e688740888da6fc5e7f55a6c8cfdc885138e` |
| `candidates/03-contexts.md` | `a3b952af54e5aa83b98eeadce38473659a9190ba9b4928e25a4665299881a79f` |
| `candidates/04-display-fences.md` | `747436ecb10ff8f848a38b1e65480d5beb26908f8d93f9ef4c40250e942b778b` |
| `candidates/05-display-dollars.md` | `932f51af7b8fa8860c5b7b92e9273e8542a41eb58a74eefbf885e8857bc4915e` |
| `candidates/06-expanded-notation.md` | `6b1af4985211f90fafdbfda24a8915651ad3c5a204ba15a072fac81286d89313` |
| `candidates/07-paper-constructs.md` | `a13238a3691817f4282298480413279a796d3cd8468b3cd96384b3b3cb164cad` |
| `candidates/08-failure-controls.md` | `5d4ad4bc58606b81a914d4c48852623149bf6a3d7edb8db547e6354371da50b3` |
| `observation.txt` | `3db14e03ea06932a13e325ea20ef65da9e617e8ac00a79556d8db8af9b6a77a7` |
| `observation-sheet.md` | `8587edbe8a8a0a53eb5efdfc8b7a61e8d1d093d78cb81e041c3adede86d6d82e` |
| `2026-11-24/flow-spacing-observation.md` | `195f2de4a7e466020afa6f6d8fdb2b9601db798899d5bbb3f2ebf176e00149ce` |
