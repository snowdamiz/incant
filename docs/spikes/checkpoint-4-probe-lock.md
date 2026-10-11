# Isolated native physics dependency closure

Checkpoint 4 is frozen at main94 `6fc63bfa1f6eafc2d0052ddc0564a8e7770b714b`.
All three native physics jobs stopped at the existing `--locked` Clippy command:
the standalone probe lockfile did not include the navigation and localization
dependencies that had become reachable from `incant_runtime`. The preceding
candidate/raw-solver controls succeeded, but the later native ECS parity and
shipping controls did not execute. Their missing-artifact errors are consequences
of this failure. The failed checkpoint is not promoted or silently retried.

Locked, offline Cargo metadata reproduced the failure locally. The other three
committed standalone probe lockfiles still resolved. Repairing this lock adds 53
package identities already present in main's lockfile and preserves all 166
previous package versions, sources and checksums. The main workspace lockfile,
engine sources, numerical settings and all existing workflow commands are
unchanged.

An independent workspace does not inherit the root workspace's crates.io source
patches. The probe now explicitly selects the same existing vendored `rerecast`
path as the engine. Without that entry, automatic resolution would select the
registry package despite the root's reviewed source patch. No dependency upgrade
or new vendor change is included.

## Fast regression boundary

The existing Python tools suite now checks required local path-dependency edges
in every committed standalone probe lock and requires reachable engine source
patches to retain their exact local path. It follows transitive required local
dependencies, workspace inheritance, renamed dependencies and target tables.
Synthetic stale-edge, missing-package and source-substitution fixtures fail as
expected; the actual main94 lock also fails this guard before the repair.

This is a structural early warning, not a replacement Cargo resolver. Optional
and external dependency resolution remains governed by the existing locked Cargo
builds. The fast gate gains no Rust compilation, dependency downloads, workflow
or required context.

## Verification

The repaired native all-target release Clippy and locked wasm32 compilation pass.
All five structural tests and the full Python suite pass (208 tests run, four
platform-specific skips). Three fresh trials compare native one-/four-worker execution with actual
Chromium 156.0.8078.4 serial WebAssembly for both 3D and 2D scenes: all 18 streams
match exactly within each scenario. All stream hashes were independently checked.
[Exact source, dependency, log and process evidence](evidence/checkpoint-4-probe-lock.json)
records the accepted review and terminal validation.

Hosted Windows/macOS/Linux confirmation belongs to the next deliberately frozen
checkpoint after the main repair. No physics adoption, phone-performance or phase
gate is claimed.
