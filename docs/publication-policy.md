# Publication policy: private references in published content · `rec:policy:publication-audit`

**Preamble (Scope)** · `pref:policy:scope`

Everything this repository publishes — code, documents, generated artifacts, history — is audited before release for references that do not belong in public. Each finding is classed by the harm its publication would cause, and the class decides the action.

|Severity|What it is|Examples|Action|
|--------|----------|--------|------|
|Critical|Secret bytes or anything that grants access|private key material, sign-in files and tokens, passwords, session credentials|The release does not happen. The commits carrying the secret are rewritten out of the history, the secret is rotated as if leaked, and the audit is rerun.|
|Major|Facts that map private infrastructure or name a person's machine|hostnames, addresses and ports of private build or development machines, account names, absolute paths carrying a personal username, private correspondence, build-log and process residue, personal email addresses other than this project's published identity|Cleaned before the release is cut: the reference is generalized (a personal path becomes a placeholder, a machine name becomes `node-a`) or the file is dropped. A release is cut only with zero major findings.|
|Minor|Project vocabulary and harmless provenance|this project's own names and label slugs, dated measurements, placeholder paths, documentation addresses from the reserved ranges|Passes. The audit lists it; the text is corrected when next touched, never by a dedicated rewrite.|
|Pass|Public by design|the project's published git identity and account, published documents and their labels, public commit hashes of public repositories|Nothing.|

**Rule (Doubt and reconstruction)** · `rule:policy:doubt-and-reconstruction`

A doubt between two classes takes the higher one, and a set of minors that together reconstruct a private layout counts as major, because the scale measures what a reader can do with the publication, not what any single line looks like alone.

**Rule (The name boundary)** · `rule:policy:name-boundary`

This project may name itself; the names of other private projects and the organizations behind them are major and are cleaned before the cut. A project's own vocabulary belongs to its public record, while another's name maps a private landscape.

**Rule (The report and the gate)** · `rule:policy:report-gate`

The audit report lists each finding with its class, file, commit and proposed generalization, and ends with counts per class. A release is cut only from content reporting zero critical and zero major findings.
