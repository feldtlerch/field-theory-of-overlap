# Display spacing in forge Markdown · `reg:documents:markdown-display-spacing-observation`

On 2026-11-24, a signed-out view in a desktop application's embedded Electron pane (Chromium 156.0.8078.100 on Linux) compared fourteen spellings in `forge-spacing-probe.md` on a test branch. That probe file was not retained in this tree. At a 1024 by 768 viewport and 16 px mathematics font size, rectangles were measured from `a` to `=` and from `=` to `b`; the unspaced display had a 4.50 px gap on each side.

| Spelling in the probe | Result in the forge's mathematics view |
| --- | --- |
| `\;` inside separate-line double dollars | A literal semicolon appeared on each side of `=`; the forge consumed the backslash before mathematics rendering. |
| `\thickspace{}` inside separate-line double dollars | The command appeared twice as red literal text. |
| `\hspace{0.2778em}` inside separate-line double dollars | A clean `mspace` added 4.44 px to each baseline gap, giving 8.94 px on each side; no command or stray punctuation appeared. |
| `\;` inside a fenced `math` block | A clean display also added 4.44 px on each side; the fenced envelope is not used. |
| `\quad{}`, `\enspace{}`, `\mskip5mu{}`, `\kern0.2778em{}` and `\mkern5mu{}` | They rendered without stray text but added 19.99, 11.99, 8.44, 8.44 and 8.44 px respectively, wider than the intended thick space. |

The subsequent forge view of `generated/documents/explainer-pilot/en.md` and `en.labels.md` on the test branch showed one display in each file, with six `\hspace{0.2778em}` occurrences each becoming a 4.44 px `mspace`. The gap before `=` measured 8.94 px, matching the probe's 4.50 px baseline plus 4.44 px. No red command text, literal semicolon or desktop overflow remained; the stop stayed inside the display. At 375 px, the display scrolled inside its container without widening the page. The observed files have SHA-256 `fafaf92129f86c1cb41559a4143c87d43d59e6e5ade7e9ebc78af379d07bd457` and `cdf33e404a3977e2ef8a0cefaba64fe046cff6fdbbc330a2132e289d688a7460` respectively.

The pane still drew Greek letters, the dot accent and underbraces as empty boxes. Greek commands and Unicode letters produced the same rendered tree in the probe, so this observation does not identify an emitted-spelling repair for those glyphs. The test-branch probe was observed at desktop width only; other engines, signed-in views, mobile applications and the right-hand operator gaps in the complete flow were not measured.
