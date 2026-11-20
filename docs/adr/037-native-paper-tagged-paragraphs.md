# ADR 037: Native paper-tagged paragraphs · `rec:adr:037-native-paper-tagged-paragraphs`

**Decision (Statement)** · `dec:documents:paper-tagged-paragraphs`

A native document may explicitly select the existing paper-scoped handle store until its replacement at the complete-graph boundary. Tag runs resolve through the permanent live resolver first, falling back only on unknown spellings to Python-checked paper or alias bindings with retained source evidence; this creates no permanent admission. Paragraphs use the paper's existing tag commands and plain handles on screen, retaining targets in annotated Markdown alone. A checked paper tag whose printed spelling is its own key may occupy that exact planned reader span even when the key also occurs in the paper mint inventory; no other reader-label exception follows. Paragraph metadata is the sole mint, and its paper wrapper expands to its body alone. Unique inclusion markers select single-line fragments and preparation refuses authored copies. The checked bridge binds strict schema-1 data to captured input bytes, preserves alias and declaration origins, and refuses unavailable, malformed, oversized or stale results.

**Reasons (Why)** · `arg:documents:paper-tagged-paragraphs-reasons`

One binding authority keeps native prose and the inherited paper consistent without requiring theory admission before complete documents. A key already printed as a handle does not become hidden machinery when its paragraph becomes native. Plain handles let this content conversion stand independently of an unobserved PDF-link form. Marker selection preserves logical inclusion when unrelated lines move.

**Declined (Alternatives)** · `arg:documents:paper-tagged-paragraphs-declined`

A second binding parser can disagree with preparation. Admitting paper objects solely to render prose couples document completeness to theory admission. Exempting a spelling throughout a reader file would hide literal-label leaks. An unused paragraph anchor has no reader purpose. Source-line selectors make wrapping a dependency of inclusion. This decision establishes neither a new font policy nor a printed-tag promotion route.

**Consequences (Accepted)** · `arg:documents:paper-tagged-paragraphs-consequences`

ADR 014:11 describes the paper-scoped store as “an exception to the two permanent stores”. Its scope includes native paper-context paragraphs that still refer to its existing targets; the store, bindings, lifetime and replacement obligation are unchanged. ADR 030's replacement of ADR 016:33 retains “no new exception is admitted”. Using the same checked store from a converted passage is not a new exception: it introduces no identity domain, new store or exception to converted-unit checks and extends no release boundary. Permanent resolution remains first and documentary targets cannot enter through the fallback. ADR 014:11's promotion sentence is unchanged. ADR 017's reader-file rule continues to exclude internal labels; the exact-span distinction recognizes a public handle in its checked role. Screen links and HTML retain their separate observation and font obligations. Complete documents in all three media remain required by ADR 030.
