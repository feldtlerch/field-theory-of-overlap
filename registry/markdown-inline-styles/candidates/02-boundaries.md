# Inline-style probe: boundaries

No style argument in the twelve body sources contains another style call. S08–S09 are constructed controls without source occurrences; the other cases cite exact source fragments.

**S07.** Source `documents/paper/explainer/explainer.tex:293`: `\textbf{capacity}: how much \emph{could} move`; proposal `**capacity**: how much *could* move`; faithful: the colon is outside bold and only “could” is italic; unfaithful: either boundary moves or punctuation disappears.

**capacity**: how much *could* move.

**S08.** Constructed control, no source occurrence: `\textbf{felt \emph{total}}`; proposal `**felt *total***`; faithful: both words are bold and only “total” is italic; unfaithful: either style is dropped or its closing mark consumes following text.

**felt *total*** remains.

**S09.** Constructed control, no source occurrence: `\emph{a \textbf{fact}}`; proposal `*a **fact***`; faithful: both words are italic and only “fact” is bold; unfaithful: one style is lost or following punctuation is styled.

*a **fact*** remains.

**S10.** Source `documents/paper/explainer/explainer.tex:90`: `\emph{helped}, \emph{untouched}, or \emph{harmed}`; proposal `*helped*, *untouched*, or *harmed*`; faithful: each comma and following space remain outside emphasis; unfaithful: a comma becomes italic or is swallowed by delimiter parsing.

*helped*, *untouched*, or *harmed*.

**S11.** Source `documents/paper/explainer/explainer.tex:306-307`: `\emph{And must --- once the sufferer's own weighing is in` followed by `hand}:`; proposal `*And must — once the sufferer's own weighing is in hand*:`; faithful: the em dash and the whole clause are italic, while the colon remains outside; unfaithful: the dash changes character or the colon joins the span.

*And must — once the sufferer's own weighing is in hand*: the total is built on no shared rod.

**S12.** Source `documents/paper/explainer/explainer.tex:302-303`: `\emph{you, by your own` followed by `caring}.`; proposal `*you, by your own caring*.`; faithful: only the scoped words are italic and the period remains outside, in the middle of a paragraph; unfaithful: the boundary moves or the period leaks as syntax.

What the event cost or paid *you, by your own caring*. This is the quiet marvel.
