# Native haptics CPU qualification

The reviewed TypeScript/desktop slice is implemented on actual main253. Source
`44b9aead` contains the final `37eaed08` correction and a docs-only main253 merge;
all compiled inputs are identical to frozen source005. Haptics changes no Cargo
manifest, dependency resolution, authoring route or static-module ABI.

The final CPU checks pass:

| Check | Result |
| --- | --- |
| Input types and existing input integration | 29 cases |
| Script library, including haptic transactions and persisted receiver guard | 152 cases |
| Script audio/HUD/authored binding integration | 20 cases |
| Composed game library/integration | 44 cases |
| Desktop window/input CPU contracts | 53 cases |
| Desktop haptics plus real static-module refusal | 6 cases, overlaps the window suite |
| SIMD8 focused script/game haptics | 10 + 1 cases, overlaps deterministic coverage |
| Strict selected-package Clippy | Pass, library and tests |
| Development Wasmtime-host compatibility | Compile pass |
| Strict TypeScript SDK, physics manifests, authoring-free dependency guard | Pass |
| Scoped Rust formatting, conventions and whitespace | Pass |

The desktop checks instantiate mock motors and test the pinned Gilrs duration
conversion. The static-module control executes an ordinary validated/metered
no-op module after capability refusal. No native window, GPU, device or physical
motor is started. The original game-module fixture expected a populated scene
and trapped when ticked against the empty refusal fixture; its failure and binary
remain retained separately.

Three failed command generations are preserved. Besides that fixture mismatch,
strict Clippy initially rejected the upstream large-error fallback closure and a
nested `if`; mechanical source003 corrections passed. The first broad script run
passed 150 cases and failed the existing pure-structural reentrant overflow case:
the combined quota had changed its error variant. Source005 preserves the original
`Structural(BufferFull)` path when no haptic/restart requests exist, and adds a
mixed reentrant control proving the shared quota still rejects before commit.
The final 152-case script run passes the unchanged original regression. Source004
was frozen but not executed; its new test identifier spelling was corrected in005.

Pending output refuses ordinary, opaque and persisted checkpoint capture or
receiver replacement before decoding/mutation. Restored VMs advertise no haptic
capability and never replay physical output. Current host generations remain the
authority. Failed stop is remembered across repeated cleanup and prevents resume
or later dispatch; drop remains best effort. Gilrs' worker remains process-lived,
and its asynchronous server channel is not a bounded or acknowledged hardware
delivery queue. Wasm haptic ABI parity and mobile actuator support remain open.
See [the API and lifetime scope](native-haptics.md).

The compact [manifest](evidence/native-haptics/manifest.json) binds 31 command
receipts (28 successful, three historical failures), 3,839 source/build inputs,
all 33 final frozen inputs and 46 immediately copied executables. Counts overlap
and must not be added into a unique total. The lossless archive contains logs,
source generations, producer inputs and the original fixture. Executables remain
local and hash-bound rather than embedded in the repository. No full workspace,
physical-delivery, mobile, performance or phase-gate acceptance is claimed.

Recheck without compilation or device access:

```sh
python3 docs/spikes/evidence/native-haptics/collect.py --verify-only
# Also verify retained local executable bytes and executable modes:
python3 docs/spikes/evidence/native-haptics/collect.py --verify-only --local-binaries
```

Root source001 review is retained in the archive. Final independent source and
execution review is pending; the sole publisher owns subsequent integration.
