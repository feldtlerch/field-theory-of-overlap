# Math probe: failure controls

The first three cases intentionally submit unavailable commands; record the failure appearance without treating it as an acceptable spelling.

**M48.** An error or intact source identifies the invented command rather than silently presenting a valid equation.

A: $\mathprobeunknown{x}$.

B: $`\mathprobeunknown{x}`$.

**M49.** An error or intact source exposes the unexpanded paper macro rather than silently presenting a valid symbol.

A: $\leanbare$.

B: $`\leanbare`$.

**M50.** An error or intact source exposes the document preamble command rather than loading a package.

```math
\usepackage{amsmath}x=1
```

**M51.** This final control still shows x subscript one, so an earlier failure has not disabled later mathematics.

A: ${x}_{1}$.

B: $`{x}_{1}`$.
