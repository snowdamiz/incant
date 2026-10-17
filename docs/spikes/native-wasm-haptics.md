# Native Wasm haptic capability parity

Native Wasm guests can read trusted haptic targets and queue typed rumble,
impact and stop requests through the Rust SDK. Both development Wasmtime and
shipped-module wasm2c hosts use the same bounded transaction owner as the native
session. This change enables the existing device owner protocol; it does not
add a device backend or establish physical vibration, shipping export or
performance results.

Additive `incant_haptics_v1` operations 66–68 expose snapshot length, snapshot
copy and command submission. ABI version 1 and all preceding signatures stay
unchanged. The SDK offers `Tick::haptic_capabilities`, `play_haptic` and
`stop_haptic`, using caller scratch or fixed stack storage. Input connectivity
alone grants no haptic authority. Every request carries a trusted target's
generation, revalidated at submission. At most 16 gamepad targets and one
device target fit the fixed snapshot; at most 32 requests fit each accepted tick.
Requests also consume the existing shared native command budget.

Wire validation rejects unknown tags, malformed framing, out-of-range memory,
stale generations and nonzero unused fields. Finite f64 amplitudes are checked
in [0, 1] before narrowing to f32. Rumble is gamepad-only with a 1–5000 ms
duration; impact is device-only; stop supports either. Raw stale epoch calls
retain existing revocation behavior, and invalid current-epoch requests poison
the transaction even if a guest ignores the error. The SDK retains its sticky
local-error behavior. A failed tick publishes neither a numeric/state prefix
nor haptic requests.

Successful requests remain pending until the owner drains or explicitly clears
them. A new tick, capability replacement, checkpoint capture or reload refuses
pending output without consuming it. Failed capture/reload attempts keep the
accepted output drainable. New or restored VMs start with neutral haptic
authority; saved data cannot restore device readiness or generations. Existing
composed session acceptance still gates PCM and requests, and suspension clears
pending output and authority. Ordinary device execution is unchanged.

## Executed scope

The frozen `c218cce4` producer passed 47 local CPU controls: four Rust SDK wire
controls, 22 shared capability/persistence/restart controls, eight actual-host
haptic controls, one legacy static numeric guest, ten existing TypeScript
haptic controls, the ordinary static player owner and the existing composed
TypeScript owner. Eight Rust SDK guest variants plus the existing bulk and
startup-refusal fixtures were translated into the linked native bundle.

Actual guest controls compare typed requests with TypeScript across both native
hosts, preserve world/input on pending-output refusal, reject stale device
generations and ignored errors, and verify checkpoint/reload neutrality. The
composed late-refusal test first demonstrates an audible accepting route and
observes the world update in the SceneHost, then requires the precise late
consumer refusal with zero PCM and hidden haptics. These are CPU protocol and
transaction results, not physical device actuation.

CPU001 passed the zero-feature strict library check and built the normal
cooker, then guest preparation refused an underscore in the externally linked
module name. The successor changed only that external name and the matching
test/helper checks; exact compiled guest inputs were retained. CPU002 passed
all 47 controls, then its broad development test lint encountered the unrelated
`INCANT_FLIPBOOK_GUEST` fixture requirement. That failed stage remains recorded.
CPU003 limits development lint to its library and the two changed haptic test
targets while retaining all selected SDK/ABI/runtime/script/player lib/test
targets. That attempt retained its separate feature-selection compile refusal:
splitting out the development test package omitted the owning game runtime UI
feature while
script UI remained selected. CPU004 restored the exact game runtime UI/native
checkpoint features observed in the passing host tests, then both strict lint
commands passed. No runtime code or acceptance condition changed. Together with
CPU001, three strict lint stages passed.

The compact data-only records bind exact producing source inventories, tool and
guest identities, command/output records, all three failed attempts and terminal
process inventories. Native binaries and full source inventories remain
external and hash-bound. Portable replay checks recorded evidence and never
executes archived code. Local full-artifact/source verification was performed
at each attempt's admission and closeout; portable records do not replace it.

No GPU, window, physical haptic device, ordinary shipping export, timing or
phase-gate claim follows from this qualification. The fixed trusted build tools
ran under sampled process-group/RSS/storage supervision, not a hostile-code
sandbox.

The four haptics attempts consumed 593.225 seconds. Including prior shipping/editor
work, the continuing target account totals 1597.580 seconds against its original
6000-second ceiling. Final and sampled peak allocation was 11,117,834,240 bytes
under the explicitly extended 12,033,904,640-byte cap. The historical baseline
and all failed attempt charges are retained. All terminal wrappers and their
observed managed PIDs/groups were checked absent before releasing the target.

[The manifest](evidence/native-wasm-haptics/manifest.json) binds 212 inert records
(382,393 compressed bytes). Seven portable controls include coherent altered
outputs/provenance, missing streams, relaxed lint, missing/empty process
inventories and omitted failure charges. The 30 focused SDK/helper portable
controls also passed. Run the bounded evidence replay without native execution:

```sh
python3 -B docs/spikes/evidence/native-wasm-haptics/verify.py
python3 -B docs/spikes/evidence/native-wasm-haptics/test_verify.py
```
