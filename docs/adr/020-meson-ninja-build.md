# ADR 020: Meson graph and Ninja scheduling · `rec:adr:020-meson-ninja-build`

**Decision (Statement)** · `dec:build:meson-ninja`

Meson 1.11.2 describes the dependency graph and Ninja 1.13.2 schedules it. Setup refuses another version of either tool. Each working checkout keeps its build directory and Cargo target directory outside the source checkout and at stable paths; Cargo remains the Rust compiler driver. NATIVE is a copy of `meson/native.ini.in` with each placeholder replaced by an absolute executable path on the executing host. Setup checks the version strings; the archive and executable digests below identify the provisioned build, while the content records bind executable bytes used by production. The Meson source archive is the official PyPI sdist with SHA-256 `698feae069cef3ecd4d7aaf281d7df359bdfcf555a9a1564383d3b913fa8a736` and installed `meson.py` SHA-256 `0250933d2cc51857a23f55bb156c1a49c0e1812e143bf9f51a80e6ce37012ff0`. The Ninja upstream `ninja-linux.zip` archive has SHA-256 `5749cbc4e668273514150a80e387a957f933c6ed3f5f11e03fb30955e2bbead6` and its installed executable has SHA-256 `607e668f90dd6cd82e1a42ae572647ad1b1fd43063964295b9547836d8c15d99`.

The round sets absolute SRC, BUILD, NATIVE, SUPPORT, NOTATION_SUPPORT, NINJA, CARGO_TARGET_DIR and CARGO_CACHE paths; BUILD and CARGO_TARGET_DIR lie outside SRC, and NINJA is the executable named in NATIVE. NOTATION_SUPPORT is the prepared notation support directory; Meson passes it to document generation and checking, the live release test and the supported regression suite. PATH resolves `meson`, `ninja`, `python3`, `cargo`, `rustc` and `node` to the same executables as the current Meson process and NATIVE; setup checks their resolved paths because the records, driver and graph checks use PATH. CARGO_CACHE is an operator-provided, preseeded offline Cargo home containing all locked registry and Git sources, shared across independent build directories. `(cd "$SRC" && CARGO_HOME="$CARGO_CACHE" cargo fetch --locked --offline --manifest-path "$SRC/Cargo.toml")` checks that cache before setup and refuses a missing dependency. Run `meson setup "$BUILD" "$SRC" --backend=ninja --wrap-mode=nodownload --native-file "$NATIVE" --prefix=/ -Dsupport="$SUPPORT" -Dnotation_support="$NOTATION_SUPPORT" -Dcargo_target_dir="$CARGO_TARGET_DIR" -Dcargo_home="$CARGO_CACHE"` once for a new build directory. Every round then runs `"$NINJA" -C "$BUILD" -j 1 meson-test-prereq` followed by `meson test -C "$BUILD" --no-rebuild --suite unit --suite product --suite regression --print-errorlogs`. A failed prerequisite build stops the round. Generation installs the pair with `meson install -C "$BUILD" --no-rebuild --tags doc --destdir "$SRC"` after the prerequisite build.

For release reproduction, choose empty absolute RELEASE_A and RELEASE_B build directories and distinct RELEASE_A_CARGO and RELEASE_B_CARGO target directories outside SRC. With the same SRC, NATIVE, SUPPORT, NOTATION_SUPPORT and prechecked CARGO_CACHE, run these commands sequentially; both product suites compare their own completed pair against the committed PDFs and receipt:

```sh
meson setup "$RELEASE_A" "$SRC" --backend=ninja --wrap-mode=nodownload --native-file "$NATIVE" --prefix=/ -Dsupport="$SUPPORT" -Dnotation_support="$NOTATION_SUPPORT" -Dcargo_target_dir="$RELEASE_A_CARGO" -Dcargo_home="$CARGO_CACHE"
"$NINJA" -C "$RELEASE_A" -j 1 meson-test-prereq
meson test -C "$RELEASE_A" --no-rebuild --suite product --print-errorlogs
meson setup "$RELEASE_B" "$SRC" --backend=ninja --wrap-mode=nodownload --native-file "$NATIVE" --prefix=/ -Dsupport="$SUPPORT" -Dnotation_support="$NOTATION_SUPPORT" -Dcargo_target_dir="$RELEASE_B_CARGO" -Dcargo_home="$CARGO_CACHE"
"$NINJA" -C "$RELEASE_B" -j 1 meson-test-prereq
meson test -C "$RELEASE_B" --no-rebuild --suite product --print-errorlogs
```

**Reasons (Why)** · `arg:build:meson-ninja-reasons`

- A declared pair target gives all paper consumers one production, and content records make retained outputs useful across rounds.
- Ninja's measured `restat` cutoff leaves downstream targets idle when refreshed records keep their bytes and mtime.
- A build directory owned by one working checkout keeps generated evidence out of the checkout, while Cargo keeps the Rust build and lockfile semantics already used by the repository.
- Exact pins and executable hashes make the graph toolchain a reviewable input under ADR 008 (`dec:publication:fresh-history`) and ADR 013 (`arg:workspace:declined`).

**Declined (Alternatives)** · `arg:build:meson-ninja-declined`

Keeping separate shell consumers preserves repeated pair production. Replacing Cargo with a second Rust dependency description duplicates ownership. A floating Meson or Ninja version makes graph behavior an unreviewed input.

**Consequences (Accepted)** · `arg:build:meson-ninja-consequences`

The retained build directory belongs to one source path and one working checkout. A new checkout path or clean release directory compiles and typesets anew. The existing `cargo run --locked -p ftoo_codegen --bin paper -- generate|check` commands remain the Meson-free route and regenerate the pair on every use. ADR 018's documentation-only gate remains separate and unchanged.
