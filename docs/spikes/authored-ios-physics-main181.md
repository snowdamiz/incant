# Authored iOS physics on the composed main181 runtime

The iOS exporter selects the authored deterministic or fast physics mode and
verifies the selected dependency graph, five actual target solver artifacts and
shipping codegen. All eight implementation/probe files remain byte-identical to
accepted producer `818ff1b2`; the original producer execution is retained separately.

Current tested source is `6b1724690f80b5fdd979dfc166b9dda0194958cc`, containing main181
PCM and restart plus the reviewed Android admission. Actual main182 ancestry was
joined at `d50e01458ff63175e2940b09cf9504964dd445b2`; that join changes documentation
only. Final integration review and the required fast check remain pending.

## Current execution

Nine commands pass on 1,963 unchanged frozen inputs. Twelve iOS tooling cases,
eight Android tooling cases and the diff check pass. Both device target checks
compile the composed player and independently validate the actual target's five
solver feature sets, excluding host artifacts.

Two fresh shipping simulator archives use the selected deterministic and fast
kernels. Their C and Swift callers complete twelve source/cache/build/map-deleted
positive processes with 864 vector assertions, 432 complete log records and exact
nonzero PCM against the native oracle. Thirty-two precise refusals include
checksum-valid opposite-mode packages in both directions, bounded arguments,
unknown versions and no-clobber behavior. Each mode retains all 100 raw command
streams and nine runtime files. Both uniquely owned simulators were shut down
and deleted after the proof.

The archive sizes are 14,581,768 bytes (deterministic) and 14,577,704 bytes (fast).
The [manifest](evidence/authored-ios-physics-main181.json) binds 506 lossless
records, 168 payloads, the archive/caller binaries and the complete source freeze.
Run `python3 docs/spikes/evidence/authored-ios-physics-main181/collect.py --verify-only`
to replay the evidence assertions without rebuilding or modifying receipts.

## Reused evidence and limits

All Rust, manifest, lockfile, SDK, schema, vendor and toolchain inputs are identical
to the reviewed Android/main181 runtime. Its 21 player/mobile cases in each mode,
Clippy, seven cooker admission cases and matched headless feature audits are
reused explicitly; these are not claimed as new Rust test runs. Fresh native
preparation uses those exact copied headless binaries and public project commands.

The collector also rehashes all 713 records, 242 payloads and 18 external binaries
of the accepted original producer. Its four shipping archives and unsigned device
C/Swift links remain **producer-scoped**. The current union has device compilation
only; it does not claim new unsigned device links or execution on physical devices.

Actual CPU execution was on the local ARM iOS 27.0 simulator. Minimum-OS execution,
physical-phone performance, rendering, mobile presentation/audio hosts, signing
and phase approval remain open. Static Wasm iOS export is a separate follow-on;
Android static modules remain refused.
