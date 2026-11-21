# Inline-style probe: structural contexts

These cases retain their source surroundings; a successful isolated candidate does not admit its enclosing table, list or display.

**S20.** Source `documents/paper/parts/appendices.tex:447`: `\textbf{\normalfont register} & \textbf{\normalfont tags} & \textbf{\normalfont gloss}`; proposal `<span style="font-weight:normal">register</span>` in the first header cell; faithful: `\normalfont` cancels bold while the three cells remain distinct; unfaithful: a bold first word or merged cells. This table-specific reset is outside this style record.

| <span style="font-weight:normal">register</span> | <span style="font-weight:normal">tags</span> | <span style="font-weight:normal">gloss</span> |
| --- | --- | --- |
| sample | sample | sample |

**S21.** Source `documents/paper/parts/appendices.tex:238`: `anchor-\emph{scarce}`; proposal `anchor-*scarce*`; faithful: the hyphen stays attached to the styled word; unfaithful: a space appears or the hyphen enters emphasis.

The anchor-*scarce* corner.

**S22.** Source `documents/paper/parts/partF.tex:134`: `\emph{self}-believed`; proposal `*self*-believed`; faithful: the hyphen follows the closing emphasis mark without a gap; unfaithful: the word separates or “believed” becomes italic.

The *self*-believed report.

**S23.** Source `documents/paper/explainer/explainer.tex:610`: `\item \textbf{Ambivalence}:`; proposal `- **Ambivalence**:`; faithful: the bold run opens a list item and the colon stays outside bold; unfaithful: the style or list boundary moves. List structure is outside this style record.

- **Ambivalence**: real stake, but the components cancel.

**S24.** Source `documents/paper/explainer/explainer.tex:215`: `\textbf{the plain rule} & \textbf{why it matters}`; proposal `| **the plain rule** | **why it matters** |` in a table header; faithful: two separate headings retain their weight and wording; unfaithful: a cell is merged or style leaks across the separator. Table structure is outside this style record.

| **the plain rule** | **why it matters** |
| --- | --- |
| sample | sample |

**S25.** Source `documents/paper/parts/front.tex:145`: `\boxed{\textbf{a state is readable at a point; a rate is not.}}`; proposed Markdown spelling: pending checked display translation; faithful: the boxed sentence retains bold and its formula association; unfaithful: the box, bold or words disappear. Display mathematics is deferred under ADR 022.

**S26.** Source `documents/paper/parts/partA.tex:189`: `\eqtag{eq:f:cap}{the gate: what \emph{could} move, $\dimratio$}`; proposed Markdown spelling: pending checked display and tag translation; faithful: “could” stays italic inside the equation-tag text and the tag remains associated with its display; unfaithful: lost style, formula or tag target. Equation-tag presentation is deferred under ADR 022.
