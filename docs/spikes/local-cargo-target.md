# Local Cargo targets with checkout ownership

Two unrelated local checkouts reused one warm Cargo target during compound integration. The failed wasm check found stale dependency metadata omitting new grouped/pipeline inputs. Preserved old metadata and an unchanged-source scoped recovery are recorded in the compound integration evidence. A successful disposable shipping build remains separate evidence.

`tools/local_cargo_target.py` gives developers an explicit target directory keyed by the resolved checkout path. It records that owner atomically, reuses an exact matching owner, and refuses copied ownership metadata, malformed markers, symbolic target links or an unowned nonempty directory. It never migrates or deletes a cache. Distinct Git worktrees sharing one repository receive different targets; symlink aliases resolve to the same checkout.

For local builds, select once from an explicit parent directory and pass the result to Cargo and any probe that invokes Cargo:

```sh
CARGO_TARGET_DIR=$(python3 tools/local_cargo_target.py --root "$HOME/.cache/incant-local-targets") || exit 1
export CARGO_TARGET_DIR
./tools/cargo test --locked --profile ci -j2 -p incant_types
```

This is opt-in developer tooling. Cargo defaults, CI caching and runtime behavior are unchanged. Do not copy another checkout's build artifacts into a selected directory. The owner stamp is not a build lock or a complete source fingerprint: keep the checkout stable while building and continue to validate actual outputs. Existing caches remain untouched; start with a fresh selection rather than relabeling an unverified target.

Six focused tests cover distinct worktrees, retained artifacts on reuse, aliases, stale/copied ownership, malformed/unowned caches, concurrent publication and CLI refusal. [Evidence](evidence/local-cargo-target.json). Independent review is pending.

[Independent source review and six-test replay](evidence/local-cargo-target-review.json) accepted this opt-in helper.
