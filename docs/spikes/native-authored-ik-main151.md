# Authored IK on current main151

The accepted authored two-bone IK producer is integrated with actual main151
`d7213ae97b4babc3559590063f925abc257d5487`. The tested source is
`897ff4067f2f171f25bb1387a0f78c11b9a0e61c`; adding current-main ancestry changed only
a root-motion review document. All 1,716 frozen source/build inputs remain exact.
Fifty-seven producer implementation/build inputs are unchanged. The two Wasm
capability/test files exactly match the accepted eight-call guard in `7eb3db1a`:
all v1–v4 legacy pose observations explicitly refuse committed IK and poison the
whole tick, while unrelated capabilities remain usable.

All 823 affected CPU cases across 138 groups pass, with 82 GPU tests intentionally
ignored. All 17 commands pass, including all-target Clippy, wasm32/static-module
checks, runtime dependency guards, 68-schema equality, generated contracts,
strict SDK types, formatting, conventions and 237 Python tests (four skips).
The [inventory](evidence/native-authored-ik-main151.json) retains raw/compressed log
identities, sources, receipts and the exact command results.

A fresh ordinary public export produced player SHA256
`7b8921ea3ad2535c03aaa4c63907d9302651b42a565c4fc3f905cfba0281ab93`
(9,268,592 bytes). Four fresh processes ran after removing authoring, build storage
and external source maps. Baseline, resumed and repeated final saves match exactly;
split PCM and logs equal the full run. Seven refusals match their precise
diagnostics, including wrong program/IK binding, corrupt/oversized input,
failed observations and create-only checkpoint publication. Fourteen actual
package files were rehashed, and all reports/save/PCM receipts are retained.
The no-clobber control preserves its existing checkpoint; PCM and checkpoint
publication are separate operations, so that control may already have written PCM.

An external receipt audit initially assumed every refusal had empty stdout. The
intentional observation assertion failure emits a failed-observation report before
refusing the checkpoint. The corrected audit checks that report and keeps empty
stdout for the other six cases. Both audit versions/logs are retained; engine,
probe and package bytes were unchanged and no shipping process was rerun.

The [producer contract](native-authored-ik.md) remains bounded: this is canonical
CPU gameplay/control/checkpoint coverage, not GPU or appearance acceptance,
edit-mode IK preview, a complete Rust/Wasm IK API, arbitrary VM persistence,
phone performance or a phase gate. Checkpoint12 is separately frozen at main150;
its actual Linux UI numerical failure is retained in this packet and assigned to
Claude0059. Release remains held until all required exact-head checks succeed.
