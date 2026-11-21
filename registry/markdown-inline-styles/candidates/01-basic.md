# Inline-style probe: source forms

Every cited source fragment below is quoted exactly; the Markdown spelling is a proposal, not an observed result.

**S01.** Source `documents/paper/explainer/explainer.tex:293`: `\emph{could}`; proposal `*could*`; faithful: only “could” is italic, with no visible stars; unfaithful: plain, bold, leaked stars or slant extending to another word.

*could* move through this edge.

**S02.** Source `documents/paper/explainer/explainer.tex:293`: `\textbf{capacity}`; proposal `**capacity**`; faithful: only “capacity” is bold; unfaithful: plain, italic, leaked stars or weight extending beyond its closing mark.

**capacity**: how much could move.

**S03.** Source `documents/paper/explainer/explainer.tex:227`: `\itshape what could move / what did move`, within the full-width `\multicolumn` row at lines 226–227; proposal `*what could move / what did move*` in that table cell; faithful: only those words are italic and the slash remains; unfaithful: a dropped slash, a slant crossing the cell, or a changed row span. The table arrangement is outside this style record.

| Comparison |
| --- |
| *what could move / what did move* |

**S04.** Source `documents/paper/parts/front.tex:5`: `{\LARGE\scshape A Field Theory of Overlap}`, within the centered title block at lines 4–10; proposal `<span style="font-variant:small-caps">A Field Theory of Overlap</span>` in a heading; faithful: the original letters read as small capitals and retain the words; unfaithful: all-capital replacement, plain text, or visible markup. Heading size and centering are outside this style record.

#### <span style="font-variant:small-caps">A Field Theory of Overlap</span>

**S05.** Source `documents/paper/parts/appendices.tex:1040`: `\texttt{nec}`; proposal `<code>nec</code>`; faithful: only “nec” is monospaced; unfaithful: leaked markup or proportional letters.

A <code>nec</code> subsection.

**S06.** Source `documents/paper/parts/appendices.tex:1044`: `\verb|\RegisterVariance|`; proposal `<code>\RegisterVariance</code>`; faithful: the backslash and name remain literal in monospace; unfaithful: command interpretation, lost backslash or leaked markup. The writer entity-escapes ampersands, angle brackets, pipes and backticks inside code.

<code>\RegisterVariance</code> supplies variance heads.
