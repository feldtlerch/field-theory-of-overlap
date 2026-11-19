# Math probe: punctuation

A uses ordinary dollar delimiters and B uses dollar-backtick delimiters; record A and B separately, including surrounding prose.

**M09.** Each formula shows x subscript one plus y subscript two, and the prose keeps its underscore.

Prose plain_name stays plain_name; the encoded control plain&#95;name should look the same.

A: ${x}_{1}+{y}_{2}$.

B: $`{x}_{1}+{y}_{2}`$.

**M10.** Each formula shows a star between x and y and a star superscript on z, while the prose word remains italic.

The *marked* word is italic and a literal \* stays visible.

A: $x*y+z^{*}$.

B: $`x*y+z^{*}`$.

**M11.** Each formula shows one half plus a visible backslash, while the prose keeps a backslash.

A prose backslash \\ remains visible.

A: $\frac{1}{2}+\backslash$.

B: $`\frac{1}{2}+\backslash`$.

**M12.** Each formula says x is less than y and y is greater than zero, while the prose angle brackets stay visible.

Prose \<sample\> and the encoded control &#60;sample&#62; stay visible.

A: $x<y>0$.

B: $`x<y>0`$.

**M13.** Each formula shows vertical bars around x, while the prose keeps a vertical bar.

Prose left | right keeps its separator.

A: $|x|$.

B: $`|x|`$.

**M14.** Each formula separates x and y with a space and then shows a tilde over x and a similarity sign before y, while the prose keeps its tildes.

Prose \~one\~ and \~\~two\~\~ keep their literal tildes.

A: $x~y+\tilde{x}\sim y$.

B: $`x~y+\tilde{x}\sim y`$.

**M15.** Each formula shows x squared plus y squared, while the prose keeps its caret.

Prose x^2 keeps its caret.

A: ${x}^{2}+{y}^{2}$.

B: $`{x}^{2}+{y}^{2}`$.
