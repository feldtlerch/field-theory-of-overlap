# ADR 002: Four-axis status and declared coverage · `rec:adr:002-status-and-coverage`

**Decision (Statement)** · `dec:graph:status-and-coverage`

Every object carries a four-axis status — maturity, identification, calibration, evidence — with values derived wherever derivable; above it, one coverage declaration names the audited slice, the closed-world flags and the release gate distinguishing a complete release from an explicit partial one.

**Reasons (Why)** · `arg:graph:status-coverage-reasons`

- Two overloaded status fields hid which axis a change moved; four independent axes make a status change say what changed, and derivation removes restatement drift between axes — the split was made once in the earlier implementation and never regretted.
- Partialness must be machine-checked state, not confession: the paper publishes before full coverage without inviting any reader to mistake the slice for the whole, and the release gate is the one bit that says which reading applies.
- A calibration status axis beside a calibration node kind was double bookkeeping; the axis stays, the kind became a payload field.

**Declined (Alternatives)** · `arg:graph:status-coverage-declined`

A single maturity scalar (collapses axes that move independently); per-document coverage statements (coverage is the authority's fact, and documents would drift from it); blocking publication on completeness (makes honesty expensive, which is how honesty ends).

**Consequences (Accepted)** · `arg:graph:status-coverage-consequences`

Status vocabulary grows only by closed-set extension; a deferred claim names what it waits on and appears in the audit report as uncertified, so the coverage boundary is visible in every surface a reader holds.
