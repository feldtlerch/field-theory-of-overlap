# Inline-style probe: identity comments and mathematics

The comments use the writer's `handle<!-- label … -->` and `handle<!-- cite … -->` forms; a proposed handle below is an observation fixture, not a resolved source identity. Styled mathematics remains refused by ADR 022 even if a candidate happens to display it.

**S13.** Source `documents/paper/parts/partF.tex:397`: `\textbf{non-dimensional source} \ftag{src:nd}`; proposal `**non-dimensional source** src:nd<!-- label src:nd -->`; faithful: only the phrase is bold, the handle remains separate and the comment is invisible; unfaithful: the handle joins bold or the comment prints.

**non-dimensional source** src:nd<!-- label src:nd -->, recovered never.

**S14.** Source `documents/paper/parts/partC.tex:258`: `\emph{De gustibus} \parencite{a:degustibus}`; proposal `*De gustibus* a:degustibus<!-- cite a:degustibus -->`; faithful: only the words are italic, and the citation handle and invisible comment retain their identity; unfaithful: the citation joins the style or the comment prints. The citation's final handle remains subject to checked resolution.

*De gustibus* a:degustibus<!-- cite a:degustibus --> forbids exactly this.

**S15.** Source `documents/paper/parts/front.tex:163`: `\emph{Existence} \handle{gen:w-exists}`; proposal `*Existence* gen:w-exists<!-- label gen:w-exists -->`; faithful: emphasis ends before the handle and its comment remains invisible; unfaithful: either boundary moves or the comment prints. The handle's final spelling remains subject to checked resolution.

*Existence* gen:w-exists<!-- label gen:w-exists -->: there is an input.

**S16.** Constructed controls, no source occurrence: `\emph{word \ftag{src:nd}}` and `\emph{word \parencite{a:degustibus}}`; proposals `*word src:nd<!-- label src:nd -->*` and `*word a:degustibus<!-- cite a:degustibus -->*`; faithful: each comment erases while its handle stays inside emphasis; unfaithful: the comment prints or a handle escapes emphasis. These constructions do not admit a nested citation or label in authored prose.

*word src:nd<!-- label src:nd -->*; *word a:degustibus<!-- cite a:degustibus -->*.

**S17.** Source `documents/paper/parts/partF.tex:398`: `\emph{instrument}: $\readout$`; proposal `*instrument*: $`R`$` as a transport fixture whose mathematical body awaits checked notation expansion; faithful: emphasis ends before the colon and mathematics; unfaithful: the formula is swallowed by emphasis or source delimiters leak.

*instrument*: $`R`$ cannot read a rate at rest.

**S18.** Source `documents/paper/parts/appendices.tex:641`: `\textbf{$g$ (the on/off)}`; proposal `**$`g`$ (the on/off)**` as a table-cell fixture; faithful: the formula and words remain within bold with the on/off phrase intact; unfaithful: math fails to typeset or its boundary moves. Styled mathematics in a table is not admitted by this probe.

| Term |
| --- |
| **$`g`$ (the on/off)** |

**S19.** Source `documents/paper/parts/partB.tex:896`: `\emph{$i$'s}`; proposal `*$`i`$'s*`; faithful: mathematics and possessive remain in one italic span with the apostrophe intact; unfaithful: the apostrophe changes or the formula fails to typeset. Styled mathematics remains refused.

*$`i`$'s* counterfactual defector.
