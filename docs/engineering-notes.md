# Engineering notes · `reg:notes:negative-knowledge`

**Preamble (What these notes are)** · `pref:notes:purpose`

Hard-won failure knowledge from the earlier private implementation, carried as named lessons the decision records cite rather than restate. Each note states the trap and the discipline that closes it; the test suites re-prove them here as the affected layers land.

- **Template engines read typesetting tokens.** A template engine can parse an expl3 parameter token as its own comment-opener, silently swallowing the rest of the file — and a *comment* mentioning the token outside a raw block does the same. Discipline: generated TeX bodies live inside raw blocks, and the generator's output is byte-gated so a swallowed tail cannot pass (`rec:adr:006-byte-gated-generation`).
- **Command declaration proves nothing.** TeX command definitions store their bodies unexpanded, so a generated command can be declared cleanly and still be unrenderable. Discipline: a machine-generated sweep typesets every generated command (`rec:adr:004-notation-ast`).
- **Token boundaries concatenate silently.** A control word emitted beside a letter forms a new, undefined command with no error at generation time. Discipline: the renderer owns token separation for every symbol at once; registry authors never hand-manage it.
- **Property-list lookups may not expand keys.** Key-value stores in the typesetting layer can require pre-expanded keys, and a key built by one rule and read by another silently misses. Discipline: the generated runtime emits explicitly expanded lookup variants rather than trusting call-site expansion.
- **Symbol names collide with the host language.** A short mathematical name can shadow a typesetting primitive (a Greek letter command, a standard operator). Discipline: the notation registry checks a proposed primitive and command against what already composes before admission.
- **Flattening fights strictness.** Some serialization frameworks cannot combine field flattening with unknown-field rejection. Discipline: rejection wins; the flattening is written out by hand (`rec:adr:011-strict-serialization`).
