# Math probe: fenced displays

These standard TeX constructs are observation candidates beyond the current closed notation primitives, not claims of admission.

**M25.** Two equations occupy separate rows with aligned equals signs, first x equals one and then y equals two.

```math
\begin{aligned}
x&=1\\
y&=2
\end{aligned}
```

**M26.** A left brace groups two cases, zero when x is negative and x otherwise.

```math
f(x)=\begin{cases}
0&\text{if }x<0\\
x&\text{otherwise}
\end{cases}
```

**M27.** A parenthesized two-by-two matrix has first row one and two and second row three and four.

```math
\begin{pmatrix}
1&2\\
3&4
\end{pmatrix}
```

**M28.** The equation x equals one has a visible equation tag reading 7.

```math
x=1\tag{7}
```

**M29.** An equation environment shows x equals one, and any automatically supplied number is recorded exactly.

```math
\begin{equation}
x=1
\end{equation}
```

**M30.** All twenty indexed terms and the final value twenty remain readable, including at a narrow viewport.

```math
{x}_{1}+{x}_{2}+{x}_{3}+{x}_{4}+{x}_{5}+{x}_{6}+{x}_{7}+{x}_{8}+{x}_{9}+{x}_{10}+{x}_{11}+{x}_{12}+{x}_{13}+{x}_{14}+{x}_{15}+{x}_{16}+{x}_{17}+{x}_{18}+{x}_{19}+{x}_{20}=20
```
