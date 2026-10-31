# ADR 013: Workspace norms — edition, lints, toolchain · `rec:adr:013-workspace-norms`

**Decision (Statement)** · `dec:workspace:norms`

The workspace uses Rust edition 2024 with a pinned toolchain recorded in `rust-toolchain.toml` and a committed `Cargo.lock`; builds run `--locked`. Clippy `all` and `pedantic` are denied through the workspace lints table; `-D warnings` is applied in continuous integration rather than in the manifest, so a toolchain upgrade that adds warnings fails CI — where it is seen and fixed — without breaking every local build. Tools are thin binaries over library logic, surfaced as checks; no limit, default or pin is raised to make a check pass.

**Reasons (Why)** · `arg:workspace:reasons`

- A new repository takes the current edition unless something argues otherwise; nothing does.
- Warnings denied in the manifest turn every toolchain upgrade into a build break for every contributor at once; the same strictness in CI catches the same defects at the gate without that blast radius.
- A committed lockfile plus `--locked` is what makes "the same inputs" true across machines and time — the premise every byte gate rests on (`rec:adr:006-byte-gated-generation`).
- The thin-binary norm keeps logic testable as library code and keeps the check surface honest: a check invokes what the library exposes, not a side path.

**Declined (Alternatives)** · `arg:workspace:declined`

Edition 2021 (no reason beyond habit); floating dependencies for freshness (freshness that changes gated bytes under you); lints relaxed to pedantic-allow (the strictness is cheap now and expensive to add later, for the same reason gates start green while empty).

**Consequences (Accepted)** · `arg:workspace:consequences`

Toolchain moves are deliberate, reviewed bites; pedantic lints occasionally demand ceremony around sound code, and a scoped allow with a stated reason is the accepted cost of keeping the default strict.
