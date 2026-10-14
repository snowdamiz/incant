# QuickJS maintained-allocation diagnostic on main272

This intake preserves exactly seven isolated diagnostic tools from `3d75ae61`
and five evidence files from `57897285`, on main272 `62a3d09f`. The reachable
cherry-picked source is `f50ad9fa`. It changes no engine, profiler, runtime,
workspace dependency or lockfile. The earlier failed Float64 store patch and
other private producer ancestry are excluded.

The producer's independently accepted Apple arm64 execution remains historical:
32 same-lock getter/full-scan observations, fourteen successful commands and one
expected wrong-count failure. The existing limit, GC, OOM and backing-owner
controls remain unchanged. This intake does not rerun native code or establish
production adoption, timing improvement, continuous peak, RSS or mutable ECS loan
safety. See [the original scope](quickjs-malloc-state.md).

## Current-main relevance and verification

All twelve imported paths match the immutable producer bytes. All six selected
workspace/toolchain inputs match main272, and all 968 lock package identities
(name/version/source/checksum, including workspace packages) match the executed
baseline. The sole full lock-row difference is main272's `incant_doc` dependency
on the already resolved `semver`; it is outside the standalone diagnostic. The
Rust toolchain, Cargo launcher and bounded native-process supervisor are exact
baseline bytes. The normal workspace still selects rquickjs `=0.14.0`.

Current checks replay the original archive, external artifacts and locally
available historical Git objects; reconcile both retained standalone locks
against the current lock; reject a corrupted registry checksum; compile the four
Python sources; and verify conventions. The initial diff check correctly found
six blank unified-diff context lines in the raw patches. That failure is retained.
Two exact-path `.gitattributes` entries preserve those bytes, and the final diff
check passes. Eight command receipts are retained; this includes the expected
negative and initial packaging failure, not eight successful commands.

The new portable wrapper binds the twelve imported paths to the reachable
cherry-picked commit and the original archive's seven frozen tools. It checks
the current dependency identity proof and original execution assertions without
requiring excluded producer ancestry. The original `--git-source` option still
requires its historical Git object; its successful local replay does not promise
that object exists in a fresh clone. `--external` requires the retained absolute
artifact paths. Neither replay executes the native binaries.

```sh
python3 docs/spikes/evidence/quickjs-getter-main272/verify.py --current --external
```

The wrapper passes with 25 current records and 22 payloads, plus the unchanged
929-record/444-payload producer archive. The original final independent receipt
`862c8924f873e3a22e14fdec6c11c56a5f18f45477c8ea28d2d60a879c321389`
is retained byte-exact. Current composition awaits independent final review.

Checkpoint33 remains separately frozen at `b3eb0a98`, exact main272 tree
`1c9687fc`; all 23 required contexts and eight comprehensive workflows must
succeed before any release promotion. Checkpoint32's Windows timeout remains a
failed historical result. This diagnostic intake supplies no release repair or
gate claim.
