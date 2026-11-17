# ADR 019: Reuse the paper pair for identical inputs · `rec:adr:019-reused-paper-pair`

**Decision (Statement)** · `dec:generation:reused-paper-pair`

The paper pair may be reused across ordinary rounds when its declared input bytes, host tools and effective arguments are identical and its completed outputs remain intact. Every round still compares the actual produced PDF and receipt bytes with the committed files, with the existing located byte difference on failure. A release, or a change to the engine, toolchain or graph boundary, forces two independent builds from empty build directories, each with its own Cargo target directory, and each passes the byte gate against the committed files. Generate and check continue to use the same driver.

**Reasons (Why)** · `arg:generation:reuse-reasons`

- Exact content records detect changed bytes even when a file keeps its old timestamp, while Ninja's unchanged-output cutoff avoids work after an equal refresh.
- The live comparison retains the byte gate's readable diagnosis for changed committed artifacts; a completion record alone does not grant acceptance.
- Independent empty builds at release and boundary changes test the reproducibility assumption on which ordinary reuse relies.

**Declined (Alternatives)** · `arg:generation:reuse-declined`

Regenerating the pair on every ordinary round repeats expensive typesetting on equal inputs. Trusting a cached verdict without comparing actual committed bytes loses the hand-edit refusal. Copying another build's result into an independent verification build loses that verification's own derivation.

ADR 006 declines content hashes as staleness markers (`arg:generation:declined`). Here a record only decides whether the pair is recomputed; it never marks a committed artifact fresh, and every round compares actual bytes with a located difference. `--built` is the same driver reading a completed pair, not a separate verifier.

**Consequences (Accepted)** · `arg:generation:reuse-consequences`

This record partially supersedes ADR 006's Decision clause “the gate regenerates and compares” (`dec:generation:byte-gated`): only for the paper pair, in a round whose paper and Rust records equal those of the retained completed pair, that pair replaces regeneration. The comparison, stale-artifact and hand-edit refusals, deterministic committed artifacts and shared generation/check path remain. On a reuse hit, the driver validates pair membership and work evidence, reads the committed and built PDF and receipt bytes, and reruns reading acceptance, registry and printed-identity checks, including the release candidate refusal when requested; the engine, clean-work preparation and PDF reader checks are not rerun. The latest result belongs to one source path and retained build directory; a new path pays a miss. The Meson-free Cargo route remains a forced regeneration through the same driver.
