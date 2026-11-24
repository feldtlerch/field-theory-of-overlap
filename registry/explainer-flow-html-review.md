# Explainer flow HTML screen review · `reg:documents:explainer-flow-html-review`

An independent screen review on 2026-11-24 examined `generated/documents/explainer-pilot/en.html` at SHA-256 `6a5911efacc64539f58d48c4a84a04043ef3df35fc216e15651d5f48846486f5` in a desktop application's embedded Electron pane, Chromium 156.0.8078.100 on Linux, at 1024 and 375 CSS pixels wide. The reviewed file was supplied at that hash; the viewer did not compute its digest. The display was compared with the admitted candidate `registry/html-mathematics/explainer-flow.html` at SHA-256 `1fb8faf3e405fef308d51357b905d967efa014e715d5201123755988d75869b3`.

| Review item | Observed value | Evidence | Verdict |
| --- | --- | --- | --- |
| Complete mathematical display and position | One `math` element with `display="block"` appears inside `div class="equation" tabindex="0"`, centred between its neighbouring notices. | Rendered page and element inspection at 1024 CSS pixels. | Pass |
| Phrase and stop inventory | The six phrases occur in order: change in overlap; will, signed; decay; reward-fed growth; surplus; witness density. The terminal stop is the last child of the display row and sits on the formula line. | Rendered display and element inspection. | Pass |
| Face and resource use | The computed mathematical family is Pagella Math; the page made zero external resource requests. | Computed family and resource entries. | Pass |
| Wide and narrow layout | At 1024 CSS pixels the container is 736 pixels wide with no overflow. At 375 CSS pixels the page remains 375 pixels wide, and the 415-pixel display scrolls within its 343-pixel container. | Container and page measurements at both widths. | Pass |
| Candidate and emitted style | The emitted page omits the candidate's `.equation-probe math{font-family:'Pagella Math',math}` rule on the container; Pagella Math was nevertheless the computed family in the observed engine. The equation scroll and focus rules are retained under the emitted class name. | Candidate and emitted stylesheet comparison; computed family. | Observed difference, no appearance defect found |
| Limits | Browser zoom, the accessibility tree, screen-reader speech, copying, other engines, and light and dark appearance were not examined. | Scope of the screen review. | Not observed |
| Overall judgment | The admitted display is complete, legible and contained in the changed explainer page at the named hash. | Rows above. | Accept for this page and engine |
