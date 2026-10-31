# ADR 011: Serialization is strict everywhere · `rec:adr:011-strict-serialization`

**Decision (Statement)** · `dec:serialization:strict`

Every serialized boundary is strict: field names in kebab-case, unknown fields rejected, duplicate keys refused, floating-point literals refused where exactness is the content (rational and integer forms carry numbers), and loaders validating byte-level concerns (encoding, key uniqueness) before structural ones. The audit's input loader and the generator's unit loader are both under this rule.

**Reasons (Why)** · `arg:serialization:reasons`

- Every laxness at a serialized boundary is a place where the checked artifact and the real artifact can disagree silently: an ignored unknown field is a typo'd field name that validates; a tolerated duplicate key is whichever value the parser kept.
- Floats refused as input is the exactness principle at the boundary: an audit of exact mathematics that accepts a float input has already approximated before it starts.
- One known trap is recorded for implementers: some serialization frameworks cannot combine field flattening with unknown-field rejection, and where that conflict arises, rejection wins and the flattening is written out by hand.

**Declined (Alternatives)** · `arg:serialization:declined`

Permissive parsing with warnings (a warning stream nobody gates on is a log of accepted defects); schema validation as a separate step (the loader that runs is the schema that holds).

**Consequences (Accepted)** · `arg:serialization:consequences`

Schema evolution is explicit: adding a field is a change to both writer and reader in one bite, and old data that no longer loads is migrated, never silently tolerated.
