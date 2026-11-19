# Math probe: surrounding Markdown

A uses ordinary dollars and B uses dollar-backticks; compare both formulas and their surrounding block.

**M16.** Each line contains two separate formulas, x subscript one and y subscript two, with the word and between them.

A: ${x}_{1}$ and ${y}_{2}$.

B: $`{x}_{1}`$ and $`{y}_{2}`$.

**M17.** Each line begins with a typeset x subscript one before the ordinary word starts.

${x}_{1}$ starts A.

$`{x}_{1}`$ starts B.

**M18.** Each line ends with a typeset x subscript one immediately after punctuation and one space.

A ends here: ${x}_{1}$

B ends here: $`{x}_{1}`$

**M19.** A two-column table shows c subscript one with an underbrace labelled pending equal to one third in both cells of its second row.

| A | B |
| --- | --- |
| $({\underbrace{{c}_{1}}_{\text{pending}}}={\frac{1}{3}})$ | $`({\underbrace{{c}_{1}}_{\text{pending}}}={\frac{1}{3}})`$ |

**M20.** Both list items show c subscript one with an underbrace labelled pending equal to one third, without exposing source syntax.

- A: $({\underbrace{{c}_{1}}_{\text{pending}}}={\frac{1}{3}})$.
- B: $`({\underbrace{{c}_{1}}_{\text{pending}}}={\frac{1}{3}})`$.

**M21.** Both headings contain a typeset x subscript one.

#### A: ${x}_{1}$

#### B: $`{x}_{1}`$

**M22.** Both centered paragraphs show c subscript one with an underbrace labelled pending equal to one third.

<p align="center">A: $({\underbrace{{c}_{1}}_{\text{pending}}}={\frac{1}{3}})$.</p>

<p align="center">B: $`({\underbrace{{c}_{1}}_{\text{pending}}}={\frac{1}{3}})`$.</p>

**M23.** Both centered paragraphs have italic prose and typeset c subscript one with an underbrace labelled pending equal to one third.

<p align="center"><em>A: $({\underbrace{{c}_{1}}_{\text{pending}}}={\frac{1}{3}})$.</em></p>

<p align="center"><em>B: $`({\underbrace{{c}_{1}}_{\text{pending}}}={\frac{1}{3}})`$.</em></p>

**M24.** Both lines show c subscript one with an underbrace labelled pending equal to one third, with invisible comments and the word remains afterwards.

A: c:1<!-- label def:langevin:coefficient-one --> $({\underbrace{{c}_{1}}_{\text{pending}}}={\frac{1}{3}})$<!-- probe annotation --> remains.

B: c:1<!-- label def:langevin:coefficient-one --> $`({\underbrace{{c}_{1}}_{\text{pending}}}={\frac{1}{3}})`$<!-- probe annotation --> remains.
