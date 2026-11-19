# Math probe: additional paper needs

These are neutral, document-macro-free candidates, not exact output promised by the current primitive registry; bodies use standard command spellings where no implemented Unicode spelling exists.

**M43.** The formula shows bold u, calligraphic W, blackboard R and an upright T superscript.

```math
\mathbf{u},\quad\mathcal{W},\quad\mathbb{R},\quad A^{\mathsf{T}}
```

**M44.** The formula shows lower-case kappa and tau, upper-case delta and pi, and the words Var and col as upright operators.

```math
\kappa+\tau+\Delta+\Pi,\qquad\operatorname{Var}(x),\quad\operatorname{col}(A)
```

**M45.** A sum from i equals one to n precedes an integral from zero to one, with both sets of limits visible.

```math
\sum_{i=1}^{n}{x}_{i}+\int_{0}^{1}x\,dx
```

**M46.** A box contains a sized parenthesized fraction with a brace underneath labelled pending.

```math
\boxed{\underbrace{\left(\frac{1}{1+x}\right)}_{\text{pending}}}
```

**M47.** Each formula shows single and double derivative primes and the word otherwise, with visibly separated final letters.

A: $f'(x)+f''(x),\quad\text{otherwise}\quad a\,b\;c\!d$.

B: $`f'(x)+f''(x),\quad\text{otherwise}\quad a\,b\;c\!d`$.
