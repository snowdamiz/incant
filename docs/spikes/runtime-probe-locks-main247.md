# Isolated runtime probe lock reconciliation

Checkpoint28's exact frozen candidate `be3422f8` failed the native physics job on
Linux, Windows and macOS before shipping/parity execution: Cargo refused to
update `tools/probes/native-physics/Cargo.lock` under `--locked`. The new runtime
checkpoint mode forwarding references snapshot through selected weak features.
Cargo must resolve that package into the lock even though the ordinary normal
graph does not activate the snapshot dependency.

An isolated copy of the failed candidate reproduced lock refusal in two of the
six nested probe workspaces. The other stale workspace was the mobile runtime
fixture, previously omitted by the one-directory-deep structural guard. That
fixture also lacked the existing profile dependency and the engine's Rapier and
Parry source patches.

## Repair

The native probe lock adds only local snapshot 0.0.1 and the three declared local
edges, retaining all 221 prior package identities. The mobile fixture adds local
snapshot/profile 0.0.1 and those edges, and uses the same checked local Rapier
0.36.0 / Parry 0.31.1 sources as the engine. It preserves the other 217 package
identities; four registry physics identities are intentionally replaced by their
same-version local patches. There is no registry version update or root lock,
production Rust, workflow, capability, or save-admission change.

The existing guard now discovers nested probe locks recursively. Cargo remains
the authority for selected feature resolution. An attempted blanket optional
local dependency rule was rejected when it incorrectly required an unselected
font dependency. Three actual tiny Cargo controls establish the distinction:
no weak reference and an inactive weak reference omit the leaf; a selected weak
reference locks it without activating it in the normal graph. The final guard
preserves that boundary and includes a negative unselected-local control. Both
initial guard attempts remain historical evidence, not passing qualifications.

## Qualification

All seventeen final checks pass: fourteen locked offline graph resolutions,
ten Python guard tests in one command, conventions and diff checks. All six
nested locks resolve on macOS; the two changed probes additionally resolve for
Linux, Windows and web, and the mobile fixture for Android and iOS. Every changed
probe's normal graph selects the exact engine-local Rapier/Parry paths and
excludes snapshot. These are resolution checks, not cross-platform compilation
or execution. No target cache, Rust compiler, shipping player or GPU was used.

The portable packet retains 63 records, all four changed source hashes and 83
build metadata/tool inputs. It preserves the actual three hosted native failures,
the copied-candidate red resolution, exact lock deltas and final graph receipts.

```sh
python3 docs/spikes/evidence/runtime-probe-locks-main247/verify.py --verify-only
```

Final independent packet review is pending. The original checkpoint stays
failed and frozen while its remaining jobs finish. Separate feature-off
NativeOwner Clippy failures are owned by a separately reviewed repair; this lock
packet does not close those failures or claim successor hosted success.
