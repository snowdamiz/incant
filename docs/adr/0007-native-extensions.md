# ADR 0007: Stable C interfaces at native boundaries

Date: 2026-10-08. Status: proposed for director review.

## Decision

Use a versioned C ABI for future native extensions; do not expose Rust layouts or Bevy World addresses. Host-owned buffers, explicit lengths and lifecycle callbacks will be required. Native code is a trusted capability, never callable by an untrusted script.

## Evidence and implementation boundary

The platform probe exports a narrow panic-contained integer-result C function linked into the iOS host and an Android JNI entry point. This validates toolchain linkage only.

## Consequences and revisit trigger

Plugin discovery, compatibility negotiation, hot unload, signing and extension permissions are not implemented. A separate ABI review is required before loading third-party code.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
