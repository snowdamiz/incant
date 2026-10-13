# Headless native session checkpoints on main246

`NativeSession::checkpoint` now composes a trusted opaque world checkpoint with
explicit script persistence, timers and captured action definitions. Reconstruction
stages a fresh VM and world; in-place restore publishes only after initialization
and validation succeed. Physical input starts neutral. Globals and closures are
reinitialized; this is not an arbitrary JavaScript heap snapshot.

The intake starts at main246 `42e13e75` and applies only source `515df482`,
producer evidence `1d604dfa` and repaired evidence `e9dc5128`. The duplicate
three-manifest repair `7a900ece` is already present through main246. Hosted
extension changes and private producer ancestry are excluded. The only
cherry-pick conflict was the current STATUS narrative.

## Source and execution generations

All **3,656** tracked implementation/build/tool files match immutable repaired
producer `7a900ece`, with the same path set, Git blobs and SHA-256 bytes. This
includes root manifests, lockfile, toolchain, Cargo configuration, every crate,
vendored dependency and tool. Two current scoped normal dependency graphs equal
the producer graphs after normalizing only the checkout prefix. Both five-solver
mode audits pass; the ordinary graph excludes snapshot and checked restoration.

The current ten checks are read-only verification, manifest/graph checks,
formatting, conventions and diff validation. There is **no new compilation or
runtime execution**. Prior execution remains explicitly scoped:

- Source004: 195 script cases in each numerical mode and 186 feature-off cases,
  both strict Clippy runs and optional/lean/wasm compilation. The original
  101-record packet retains all 18 receipts and five immediate library binaries.
- Repaired source005: nine focused native checkpoint cases in each mode, the
  manifest guard and both selected graphs. Its 11-record packet retains five
  successful receipts and two immediate library binaries. These tests overlap
  the broad counts.
- The original oracle failure, format-brace failure and manifest-policy refusal
  remain retained. The last is closed by the reviewed manifest repair, not by
  rewriting the original result.

Only library executables were copied by the producer. Integration and doc tests
have retained logs, not retained executable claims. Independent producer reviews
`a0ebc104` and `dbdf947c` are copied byte-for-byte in the current evidence folder.
Final current-main independent review is accepted (`12de683d`); PR247 merged after its exact-head required fast SUCCESS.

## Bounds and exclusions

The checkpoint is opt-in and process-local, with no byte importer. Failed hosts,
pending restart requests, incompatible programs/worlds, and current or sticky
UI/audio obligations refuse explicitly. Script values, timers and attachments
retain their existing bounds; physics work/payload/allocation budgets are not a
process-wide allocator cap. Supplied budgets may be consumed by failed staging;
the live session remains unchanged. Success changes world identity/incarnations,
so external queued work and hosts must be discarded. No Wasm, hosted audio/UI,
exported player, GPU/device or public persisted physics admission is claimed.

## Reproducible review

```sh
python3 docs/spikes/evidence/native-session-main246/verify.py --verify-only --local-binaries
```

The verifier binds both producer generations, local immediate binary copies,
current source equality and 20 retained current records. Omitting
`--local-binaries` omits the two repaired-generation local binary rehashes; the
unchanged original collector still checks its five original local copies. It
never compiles or executes a test binary. This packet is CPU/source scope only.

Checkpoint27 ended unpromoted with 22 required SUCCESS and one configured macOS
timeout cancellation. Checkpoint28 is separately frozen at main246 on PR232;
this intake cannot move that running candidate or approve a release gate.
