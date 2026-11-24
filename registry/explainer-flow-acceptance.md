# Explainer flow acceptance sheet · `reg:documents:explainer-flow-acceptance`

The reviewed surfaces are `generated/documents/explainer-pilot/en.md` at SHA-256 `fafaf92129f86c1cb41559a4143c87d43d59e6e5ade7e9ebc78af379d07bd457`, `en.labels.md` at `cdf33e404a3977e2ef8a0cefaba64fe046cff6fdbbc330a2132e289d688a7460`, and `en.html` at `6a5911efacc64539f58d48c4a84a04043ef3df35fc216e15651d5f48846486f5`.

The PDF pair has SHA-256 `1744907b8e65e7cc6129bc5db94c1ba83bbc2751854c4eb0d74333439ffc17f8` for the paper and `73594d6cdc321098c212f3a557277c23a6a1a59a2a1c7fec159b4e03f9954fee` for the explainer; both equal the parent pair byte for byte.

The revised HTML display candidate is `registry/html-mathematics/explainer-flow.html` at SHA-256 `1fb8faf3e405fef308d51357b905d967efa014e715d5201123755988d75869b3`; its filled sheet admits this display for the explainer screen only, and the emitted page has its own [screen review](explainer-flow-html-review.md).

| Review item | Expected surface | Observation | Evidence | Judgment |
| --- | --- | --- | --- | --- |
| Reader Markdown flow | Six lines preserve the four paired braces, all words, the three inherited symbols and the terminal stop in one display. |  |  | Pending |
| Annotated Markdown flow | The same display follows its owning unit comment and retains its exact attachment order. |  |  | Pending |
| Forge mathematics appearance | The display is complete and legible; known missing brace and Greek glyph behavior is recorded rather than mistaken for lost source. |  |  | Pending |
| HTML flow display | The flow replaces its unavailable block with the admitted MathML display, including the terminal stop, while other gaps remain registered. | The display is present at the flow position and the stop ends its formula line. | [Independent screen review](explainer-flow-html-review.md) of `en.html` at SHA-256 `6a5911efacc64539f58d48c4a84a04043ef3df35fc216e15651d5f48846486f5`. | Accept for this page and engine |
| Neighbourhood | Surrounding prose, headings and refrain destinations retain their order. |  |  | Pending |
| Narrow screen | The reader and annotated display can be inspected at 375 pixels without confusing truncation with omitted source. |  |  | Pending |
| Overall judgment | State a verdict only after examining the candidate surfaces at the hashes above. |  |  | Pending |
