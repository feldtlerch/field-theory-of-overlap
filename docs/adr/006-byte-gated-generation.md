# ADR 006: Byte-gated generation, one code path for generate and check · `rec:adr:006-byte-gated-generation`

**Decision (Statement)** · `dec:generation:byte-gated`

Every generated artifact — the documents, the notation files, the runtime projection that carries the authority into the typesetting run, the audit report's verdicts — is deterministic from its declared inputs and byte-gated: the gate regenerates and compares, a stale artifact fails with a readable diff, and a hand edit to a generated file is a build failure. Generated files are few, with one role each, and the runtime projection carries every edge of the graph, so "what rests on this" is answerable from the generated artifact without the source tree. The generator's check mode shares one code path with generation.

**Reasons (Why)** · `arg:generation:reasons`

- A byte comparison with a diff says *what* changed where a hash or timestamp says only *that*; the no-hashes rule survived every schema generation of the earlier implementation on this argument.
- Determinism is what makes the gate meaningful: an artifact that may legitimately differ between runs turns its gate into noise, and a gate on noise teaches people to ignore gates — hence the audit report's split into gated verdicts and ungated measurements.
- One code path for generate and check, because a checker that re-implements the generator drifts from it: the consumer-validates-producer principle applied to the toolchain itself.
- One role per generated file, because a reader must know which file answers which question; mixing roles re-creates the opacity generation was meant to end.

**Declined (Alternatives)** · `arg:generation:declined`

Generating at build time without committing (a reader of the repository then cannot read the paper); content hashes as staleness markers (no diff, no identity); a separate verifier tool (drift by construction).

**Consequences (Accepted)** · `arg:generation:consequences`

Generated artifacts live in the history and enlarge diffs — accepted, because the diff of a generated file under review is evidence, not clutter; and every source of nondeterminism (map orderings, locale, timestamps) is a generator defect by definition, found by the gate's first honest failure.
