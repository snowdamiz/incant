# Optional GPU timestamp capability contract

Checkpoint21's exact `51d8ab52` checks job failed on hosted Metal's Apple
Paravirtual adapter. The timestamp constructor correctly returned typed
`GpuTimingError::Unsupported`; its ignored numeric test unconditionally unwrapped
that optional capability. The original complete hosted log remains retained.

The test now constructs the ordinary renderer through the existing exact-backend
assertion and records the actual adapter and advertised feature bits. Without
`TIMESTAMP_QUERY`, it requires that exact typed refusal, completes an ordinary
render submission, and verifies disabled/empty timing state. This is executed
negative capability coverage, with no positive timing measurement claim.

On capable adapters, initialization must succeed on the same adapter/features.
All original bounded-capacity, abandonment, reversed submission-order, callback,
positive-duration and reuse assertions remain unchanged. Any other construction
error fails. No production code, shader, feature request, workflow, required
context or skip selection changes.

The source remains frozen across 94 passing renderer CPU cases, 36 GPU cases
ignored, strict all-target Clippy, formatting, conventions and diff checks. A
copied executable and all original logs/source identities are retained. No local
GPU execution was requested. The successor release checkpoint must execute the
actual native contract on its selected backends before promotion.

[Machine-readable evidence](evidence/timestamp-capability.json) and its read-only
verifier bind all 3,065 inputs. Independent source and final
packet reviews are accepted; publication awaits the required fast check. No device, performance or phase gate is approved.
