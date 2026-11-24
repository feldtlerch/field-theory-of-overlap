# ADR 051: Thick spaces in forge Markdown displays · `rec:adr:051-markdown-display-spacing`

**Decision (Statement)** · `dec:documents:markdown-display-spacing`

A top-level mathematical display in forge Markdown keeps the separate-line double-dollar envelope and spells each expanded-source thick space `\;` as `\hspace{0.2778em}` inside it. The reader and annotated files use the same emitted body. The common checked expression, its native TeX spelling and the source from which HTML presentation is derived retain their pinned spelling. The complete Markdown body, including its terminal stop, is checked by the strict renderer in display mode; source comparison declares this exact spelling correspondence, and recovery compares the emitted body with the planned one. No other command replacement or display context follows from this spelling.

**Reasons (Why)** · `arg:documents:markdown-display-spacing-reasons`

The forge consumes the backslash of `\;` before passing a double-dollar display to its mathematics renderer, leaving visible semicolons. It draws the letter-named thick-space command as red literal text. The observed dimensioned command adds 4.44 px to each side of an operator over a 4.50 px baseline, matching the intended thick space without stray text; the complete explainer flow retained that width without desktop overflow (`reg:documents:markdown-display-spacing-observation`). Keeping the replacement at the Markdown boundary preserves one checked expression for all renderers and gives the emitted bytes their own strict check.

**Declined (Alternatives)** · `arg:documents:markdown-display-spacing-declined`

A fenced mathematics block also rendered the short command cleanly, but it changes the admitted double-dollar envelope. Wider commands change the intended gap. Leaving the short command or the letter-named command in that envelope leaves visible error text. Changing the common notation spelling would also change the other renderers' source without evidence for doing so.

**Consequences (Accepted)** · `arg:documents:markdown-display-spacing-consequences`

The dimensioned command is a pinned Markdown display spelling for the source thick-space command, not a new notation primitive. Existing inline mathematics and HTML presentation remain under their admitted profiles. A future display spelling or envelope requires its own checked correspondence and recorded forge observation; a body that fails the strict display check or exact recovery refuses.

**Amendment (Record text)** · `amend:documents:markdown-display-spacing`

The following accepted clauses are qualified for the one observed Markdown display spelling. Their other sentences and requirements remain in force.

| Earlier clause | Replacement governed by this record |
| --- | --- |
| ADR 017:9, first sentence: “Markdown and HTML mathematics must use fully expanded KaTeX source without document-specific macros, using Unicode characters where the pinned strict renderer accepts them and the pinned command spellings otherwise.” | Markdown and HTML mathematics use fully expanded KaTeX source without document-specific macros, using Unicode characters where the pinned strict renderer accepts them and the pinned command spellings otherwise; the checked Markdown display body replaces only the expanded source's thick-space command with the observed dimensioned spelling fixed here, while the common expanded source and HTML derivation retain the pinned command spelling. |
| ADR 022:5, display sentence: “Display mathematics uses double dollars on separate lines in a top-level block with blank lines around it.” | Display mathematics uses double dollars on separate lines in a top-level block with blank lines around it; inside that envelope, each source thick-space command `\;` is emitted as `\hspace{0.2778em}` and the complete emitted body is checked in strict display mode. |

ADR 042's statement that ADR 017's first sentence continues to govern the common expanded source remains true: this qualification is confined to the emitted Markdown display body (`rec:adr:042-html-mathematics`). ADR 022's other context and boundary refusals remain.
