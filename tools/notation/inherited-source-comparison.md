# Inherited notation source comparison · `reg:notation:inherited-source-comparison`

These seven source-to-expression pairs are admitted inherited catalogue entries. Each quoted block reproduces the complete defining source bytes at its cited location; dependency closure distinguishes a source command from a typed operation. Their strict screen forms pass the generated sweep; document consumption remains unobserved.

## Comoving field · `entry:notation:comovfield-source`

Source: `documents/paper/data/ftoo_symbols.tex:53–58`.

```tex
\NewDocumentCommand
  { \comovfield }
  {  }
  {
    \ensuremath{S}
  }
```

Arity: 0. Complete visual dependency closure: the literal `S`; `\ensuremath` is the source wrapper and contributes no mathematical operand. Admitted checked expression: `Literal("S")`. Checking interface: `\RegisterVariance{\comovfield}{operator}` at `ftoo_registers.tex:83`; the named paper invocation must retain that registered operator variance. Later graph-binding target: `sem:field:co-movement-field` in `ftoo_manifest.tex:9400–9401`. Strict screen body: `S`; the checked pair is admitted and passes the strict sweep, while its first document context remains unobserved.

## Readout · `entry:notation:readout-source`

Source: `documents/paper/data/ftoo_symbols_historical.tex:21–26`.

```tex
\NewDocumentCommand
  { \readout }
  {  }
  {
    \ensuremath{\symcal{M}}
  }
```

Arity: 0. Complete visual dependency closure: literal `M` and the source's `\symcal` call, which requires a reviewed typed calligraphic alphabet primitive; `\ensuremath` is a wrapper. Admitted checked expression: `Call(Calligraphic, [Literal("M")])`, using the admitted calligraphic primitive. Checking interface: no quantity, metric or registered-variance declaration was found for this name in `ftoo_registers.tex`; the ordinary named source invocation still needs source comparison. Later graph-binding target: `sem:named:readout` in `ftoo_manifest.tex:9728–9729`. Strict screen body: `\mathcal{M}` which passes the strict parser; its glyph match with the paper source remains unobserved, so it is not admitted to a document.

## Will virtue · `entry:notation:willvirtue-source`

Source: `documents/paper/data/ftoo_symbols_historical.tex:621–626`.

```tex
\NewDocumentCommand
  { \willvirtue }
  {  }
  {
    \ensuremath{\omega\sp{\mathrm{vir}}}
  }
```

Arity: 0. Complete visual dependency closure: the Greek omega source command, the source superscript helper `\sp`, and `\mathrm{vir}`; `vir` is carried by a reviewed nullary upright primitive, not a permissive literal. `\ensuremath` is a wrapper. Admitted checked expression: `Call(Superscript, [Call(Omega, []), Call(UprightVir, [])])`, using admitted primitives. Checking interface: no quantity, metric or registered-variance declaration was found for this name in `ftoo_registers.tex`. Later graph-binding target: `sem:will-component:virtue` in `ftoo_manifest.tex:9924–9925`. Strict screen body: `ω^{\mathrm{vir}}`; its strict parsing is checked, while glyph and document context remain unobserved.

## Generator symbol · `entry:notation:gensym-source`

Source: `documents/paper/data/ftoo_registers.tex:116–117`.

```tex
\DeclareQuantity{\gensym}{0}{ dim={\Time[-1]}, variance=operator, spends={} }
  { A\sb{\mathrm S} }
```

Arity: 0. Complete visual dependency closure: literal `A`, source subscript helper `\sb`, and one upright `S` operand through `\mathrm`; there is no inherited symbol-definition dependency. The checking declaration additionally depends on the existing dimension expression `\Time[-1]` and its registered quantity parser; that dependency must be preserved by name. Admitted checked expression: `Call(Subscript, [Literal("A"), Call(UprightS, [])])`, using the admitted upright primitive. Checking interface: `\DeclareQuantity` under the exact name `\gensym`, arity 0, dimension `\Time[-1]`, operator variance and empty spend set; a visual replacement cannot bypass its named quantity call. Strict screen body: `A_{\mathrm{S}}`; its strict parsing is checked, while glyph and document context remain unobserved.

## Alignment rate · `entry:notation:alignrate-source`

Source: `documents/paper/data/ftoo_symbols_historical.tex:229–234`.

```tex
\NewDocumentCommand
  { \alignrate }
  {  }
  {
    \ensuremath{\dot{\sigma}}
  }
```

Arity: 0. Complete visual dependency closure: `\dot` applied to `\sigma`; `\ensuremath` is the source wrapper. Admitted checked expression: `Call(Dot, [Call(Sigma, [])])`. Checking interface: no quantity, metric or registered-variance declaration was found for this name in `ftoo_registers.tex`. Later graph-binding target: `sem:named:alignment-rate` in `ftoo_manifest.tex:9264–9265`, with canonical lifecycle. Strict screen body: `\dot{σ}`; its strict parsing is checked, while glyph and document context remain unobserved.

## Net will · `entry:notation:willnet-source`

Source: `documents/paper/data/ftoo_symbols_historical.tex:189–194`.

```tex
\NewDocumentCommand
  { \willnet }
  {  }
  {
    \ensuremath{\omega\sb{\mathrm{net}}}
  }
```

Arity: 0. Complete visual dependency closure: `\omega`, the source subscript helper `\sb`, and `\mathrm{net}` through the reviewed upright-net primitive; `\ensuremath` is the source wrapper. Admitted checked expression: `Call(Subscript, [Call(Omega, []), Call(UprightNet, [])])`. Checking interface: no quantity, metric or registered-variance declaration was found for this name in `ftoo_registers.tex`. Later graph-binding target: `sem:reading:net-alignment-coordinate-forcing` in `ftoo_manifest.tex:9896–9897`; this target has alternate-rendering lifecycle, unlike the canonical alignment-rate and will-decay targets. Strict screen body: `ω_{\mathrm{net}}`; its strict parsing is checked, while glyph and document context remain unobserved.

## Will decay · `entry:notation:willdecay-source`

Source: `documents/paper/data/ftoo_symbols_historical.tex:653–658`.

```tex
\NewDocumentCommand
  { \willdecay }
  {  }
  {
    \ensuremath{\gamma\sigma}
  }
```

Arity: 0. Complete visual dependency closure: `\gamma` immediately followed by `\sigma` with no authored separator; `\ensuremath` is the source wrapper. Admitted checked expression: `Call(TightJoin, [Call(Gamma, []), Call(Sigma, [])])`, with the join part of this reviewed pairing rather than inferred from incidental parser whitespace. Checking interface: no quantity, metric or registered-variance declaration was found for this name in `ftoo_registers.tex`. Later graph-binding target: `sem:dynamics:will-decay-term` in `ftoo_manifest.tex:9876–9877`, with canonical lifecycle. Strict screen body: `γσ`; its strict parsing is checked, while glyph and document context remain unobserved.

For each entry, admission compares the decoded body and full closure to these bytes, retains the named checking route and validates the strict screen body; a converted document scope separately observes its consuming context. A changed operand, dependency, arity or declaration refuses comparison; an unsupported primitive or document context leaves that scope unconverted.
