# Native numeric undo capacity reuse

Source `421018733c270ae44b7ce6e5068479b4cec9587c` starts at main276
`78d41fdb2d87546f8a7b93a4e095deaa3693224a`. Its five Rust paths have independent
source acceptance, freeze SHA256
`b4f3119ed2a0613a4cc0780dc4ec5dd889593dc1f55e0fd3c6493b71dc70d0ec`
and review SHA256
`21474c611a04280ce9861f1a91e0df5fdcb8b47785b0b0a1017719ff4f8810b0`.

Each ScriptHost now retains exact-size native ID and numeric undo allocations.
A new tick copies every current ID/value again. Matching shapes reuse their
backing allocations; shape changes evict uncaptured tails before allocating
replacements. An exclusive scratch lease spans Loan and native Undo. It returns
only after views are detached and writes are committed or restored. The cache
contains no world pointer, JavaScript root or guest view. Fresh foreign views,
complete layout/identity checks, finite-value validation and rollback remain.
Initialization and checkpoint replacement start with empty scratch; dropping
the host destroys it. These are ephemeral caches, never checkpoint contents.

The existing 32 MiB ceiling covers numeric saved copies. IDs, container metadata
and the existing JavaScript label cache remain additional. Exact-size buffers
and tail eviction/trimming prevent accumulation across topology churn. The
latest admitted scratch stays allocated while its host is idle; this is a
retention change, not a claim of lower memory use. The script compatibility
identity now also hashes loan.rs, loan/ids.rs and loan/scratch.rs. Actual build
outputs register all three, so subsequent loan changes require a fresh cook.

## Executed source checks

Rust 1.99.0, Apple arm64, CI profile, locked/offline Cargo, jobs2, Rayon2 and two
test threads were used in the released existing font target. Each complete
script suite passed 233 checks in deterministic and four-lane fast physics,
including native checkpoints, grouped bindings, profile fixtures, animation,
flipbook and spatial transactions. Each run intentionally ignored the existing
CPU diagnostic; neither execution is shipping performance evidence.

Seven new native controls passed in each mode: actual backing-allocation reuse
with current-value rollback after callback/numeric/structural failures; same-size
replacement identities; topology growth/shrink/empty storage and stale views;
partial constructor refusal; last-identity rejection before any restored prefix;
native unwind detachment and rollback; checkpoint/host replacement and destruction.
The constructor refusal uses a reduced private test budget; production new still
enforces the full 32 MiB ceiling and cannot raise it.

Strict all-target Clippy passed for both tested modes. Compiler-disabled library
Clippy passed with profiling disabled and enabled. Formatting and diff checks
passed. Eighteen final test executables (277,607,600 bytes) were retained and
hashed separately; they are CI-profile tests, not shipping players.

The first pre-freeze run passed 149 checks and failed one new stale-view test:
reading detached DataView.byteLength throws. Its raw failure and input receipt
are retained. The control was corrected to inspect view.buffer.byteLength; the
pre-freeze rerun passed 150, followed by the final checks above. No production
failure was discovered and no failed attempt is relabeled as passed.

## Replay and remaining work

```sh
python3 -B docs/spikes/evidence/quickjs-loan-capacity/verify.py
```

The small portable packet rehashes source bytes, raw logs, exact commands,
compiler identity outputs and independent acceptance, retaining the initial
failure. Add `--originals --binaries --source-root /path/to/incant` to check the
recorded external originals/test copies and current five source files. Replay
only reads files; it never builds or executes native code.

This work did not execute a player, shipping export, full Core route or timing
sequence. Other platforms and unlisted UI/audio/static-module/Wasmtime feature
combinations remain outside this qualification. The previous main275 maintained
measurements remain **2 ms gate failures**; these source checks establish no
speedup, new shipping memory result or phase approval. The completed measurement
checkout, package and 74 authored inputs were preserved. Main277 intake and any
new matched shipping execution belong to the integration owner.

## Main277 composition

The integration onto main277 (`38c0178a60ae9021a1c16787bfba39a8249aabcb`)
preserves all five reviewed Rust files and the portable packet byte for byte.
The main276-to-main277 delta contains no crates, Cargo manifests/lock, Rust
configuration, TypeScript SDK or Cargo-wrapper changes. Root replayed the packet
with external originals, all 18 retained test binaries and the current source
hashes; it passes without rerunning or relabeling the producer executions.
Generated conventions, formatting and whitespace checks pass. The first direct
`cargo fmt` invocation could not find Cargo on this shell's PATH; the repository
wrapper with the pinned 1.99.0 toolchain passes. This was a command-launch failure,
not a compiler or test result. No shipping measurement was made during intake.
