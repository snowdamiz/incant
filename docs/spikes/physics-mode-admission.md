# Compiled physics selection and cooked-scene admission

This implements the process-wide build boundary for the plan's deterministic and
fast physics modes. Per-project settings, automatic Play process selection and
export routing are separate work; no project selector is claimed by this slice.

The 2D and 3D native kernels retain parallel stepping. Default package builds
remain deterministic. Disabling package defaults selects hardware-math fast;
`physics-fast-simd8` selects the separate eight-lane candidate. No target adopts
that candidate automatically or claims a measured speedup. Browser targets retain
the serial solver. The feature graph explicitly forwards the choice through all
32 affected packages, including build, development, optional and target-specific
edges, without activating otherwise optional dependencies. A compile-time check
requires the 2D and 3D kernel constants to agree.

The checked physics enum lives with shared scalar types. The runtime exposes the
exact compiled variant and a typed family-admission check. It does not depend on
authoring documents or select Rapier behavior dynamically.

The existing 72-byte cooked-scene header is unchanged in size. The low 16 bits of
its format word retain the historical record layout (1–19); the high 16 bits bind
deterministic (0), four-lane fast (1) or eight-lane fast (2). Deterministic scenes
therefore keep their historical bytes. Unknown format words fail explicitly.
Integrity covers the complete word. Once framing and integrity pass, an exact
variant mismatch refuses before record allocation or world construction. A scene
admitted to one kernel cannot silently acquire another kernel's semantics.

Native saves contain a complete cooked scene and inherit this admission boundary.
A regression changes its variant and recomputes both integrity layers: decoding
and restoring must return the precise variant mismatch, and the live world's
bytes and tick must remain unchanged. This is not production physics save
support: worlds which used physics still refuse saving as before. The separate
trusted-state diagnostic has not supplied an untrusted solver-state decoder.

`tools/physics_features.py` audits the full repository forwarding graph and the
actual five pinned solver/math dependency records produced by Cargo. Target
records must belong to the requested target/profile directory; build-host
records cannot satisfy a missing target record. Missing, conflicting or failed
build records refuse. The manifest audit follows aliases and inherited feature
unions to catch accidental deterministic defaults in a fast build.

Local validation is retained in [the execution manifest](evidence/physics-mode-admission.json).
All 249 runtime behavior tests passed independently in each of three builds.
Headless, player and editor all-target compilation and runtime Clippy also passed
in each, with actual target-feature receipts. Each build additionally passed 105
cook/game-runtime/render-host consumer tests, five focused variant/save tests and
consumer Clippy. The 19 feature-guard tests and repository manifest audit passed.
All final commands terminated successfully with unchanged source freezes; the
manifest retains original failures from stale test expectations and a duplicate
test helper, followed by corrected successful runs. No production admission was
relaxed to make those tests pass.

The production/manifests and feature audit received independent source review;
final integration on current main remains a separately verified step. The final
consumer run includes the later nested-save regression and exact format helpers,
while the earlier complete runtime run has 249 tests. None of these local checks
satisfies reference-phone, performance, backend-adoption, full save or phase gates.
