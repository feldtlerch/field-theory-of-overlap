# ADR 003: Tags are human-facing handles bound to labels · `rec:adr:003-tags-bound-to-labels`

**Decision (Statement)** · `dec:tags:bound-to-labels`

A tag is a short human-facing handle on a theory object, bound internally to exactly one label — the object's semantic identifier — in one registry the authority owns, covering every labeled object: quantities, claims, rulings, evidence links. Every consumer works on the bound label. Tags carry a three-state lifecycle (live, retired, out of slice), never re-bind, and the registry records retirements. The generated runtime keeps retired and out-of-slice tags in separate stores with separate messages, so every tag-facing position fails the build on an unresolvable tag with a refusal naming the tag and its state; rendered Markdown displays tags naturally in running text and carries labels in HTML comments at their sites.

**Reasons (Why)** · `arg:tags:reasons`

- Labels answer "where is this said?"; tags answer "what is this thing?" — a tag survives every restatement of the text that mentions it, which no document-position mark can.
- Binding to a label makes the tag carry nothing of its own — no kind, no status, no parseable meaning. A tag's prefix (`J:`, `c:`) is ergonomic, a reminder for the reader, and nothing may parse it; being non-semantic is exactly why spellings stay stable, since a handle's value is recognition.
- Never re-binding keeps old texts and retained audit reports meaning what they meant; the three lifecycle states exist because a retired tag, an out-of-slice tag and a typo need three different fixes, and collapsing any two produced real confusion in the earlier implementation.
- A soft fallback on an unresolvable tag turns an addressing error into typeset misinformation — the worst outcome available to a self-auditing paper.
- Labels hide in the rendered Markdown because they are the machine-facing layer: present in the file so tooling reads the same addressable surface from the rendered document as from the source, invisible to the reader because they are machinery, not text.

**Declined (Alternatives)** · `arg:tags:declined`

Tags as a second label namespace (answers the wrong question); prose-minted tags (theory objects exist before and independent of the places that discuss them, so the authority mints); per-register addressing schemes (a second place to disagree); re-binding for cheap fixes (silent meaning changes in every old text).

**Consequences (Accepted)** · `arg:tags:consequences`

The registry grows with the referent expansion and is the migration's measure — the earlier implementation's prose addressed several times as many tags as its authority bound, and closing that gap is theory coverage, not data entry. Tag spellings change only for a reason about the handle itself.
