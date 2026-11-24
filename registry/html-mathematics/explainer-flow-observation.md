# Explainer flow display observation · `reg:documents:html-explainer-flow-observation`

The observed candidate page is [explainer-flow.html](explainer-flow.html), SHA-256 `1fb8faf3e405fef308d51357b905d967efa014e715d5201123755988d75869b3`; it typesets the source and its terminal stop together through the pinned strict MathML adapter in display mode and retains the full embedded faces of the HTML mathematics probe. Its source annotation includes the terminal stop, so recovery compares the annotation with the planned expression followed by that stop. No mathematical node or spacing is inserted after the adapter's output. The multiplication sign remains tight to its neighbouring words as in the delivered PDF; the scroll container accepts keyboard focus with a visible outline.

This observation was recorded on 2026-11-24 from an independent look in a desktop application's embedded Electron pane, Chromium 156.0.8078.100 on Linux. Its accept verdict covers this candidate and context; one engine was examined, with no screen reader or light and dark comparison.

| Observation | Value |
| --- | --- |
| Browser engine, version and operating system | Embedded Electron pane, Chromium 156.0.8078.100 on Linux. |
| Desktop viewport and device pixel ratio | 1024 × 768 CSS pixels; device pixel ratio 1.667. |
| Phone viewport at 375 CSS pixels and device pixel ratio 2 | 375 × 812 CSS pixels; device pixel ratio 2. |
| All four underbraces attach to their intended bases and annotations | Four nested `munder` groups attach dotted sigma to change in overlap, subscripted omega to will, signed, gamma sigma to decay, and reward-fed growth to surplus times witness density. |
| The six phrases read exactly: change in overlap; will, signed; decay; reward-fed growth; surplus; witness density | All six phrases were read exactly in the displayed tree. |
| Plain words use the embedded prose face; mathematical operators and symbols use the embedded mathematics face | Phrases computed to Pagella; mathematical symbols computed to Pagella Math. Pagella regular, Pagella bold and Pagella Math loaded. |
| Dotted sigma, subscripted omega net, and gamma joined to sigma remain distinct and legible | All three forms were present in the inspected display; no separate legibility measurement was recorded. |
| The equals, minus and plus boundaries and multiplication sign preserve their intended order and spacing | The complete flow appeared in order. The multiplication sign had no `lspace` or `rspace` attributes and measured zero-pixel gaps on both sides; the other operator gaps were not individually measured. |
| Text-space rendering inside and beside mathematical symbols, including the adapter's nonbreaking spaces, keeps the intended word boundaries | The six phrases were read with their intended word boundaries; individual nonbreaking-space behavior was not measured. |
| The terminal full stop remains part of the display and is legible | The stop was the last child of the display's `mrow`, on the formula line, with no following node in the container. |
| At 375 CSS pixels the display scrolls inside its container without widening the page or clipping text | The page measured 375 CSS pixels; the 415-pixel display content scrolled within its 343-pixel container. |
| Keyboard focus has a visible outline on the page's interactive control | The scroll container received keyboard focus and showed a solid 3-pixel outline. |
| The page makes no external resource request and all required fonts load from embedded data | Resource entries: zero; all three declared faces loaded. |
| The accessibility tree exposes the display and its text, including any mathematical role | Not observed; neither the accessibility tree nor screen-reader speech was inspected. |
| Text selection and copying preserve readable phrase and operator order | Not observed. |
| Verdict and presentation limits | Accept for the named candidate and engine. Other engines, browser zoom, screen-reader speech, and light and dark appearance were not examined. |
