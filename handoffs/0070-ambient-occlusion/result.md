# 0070 result — combined SSAO + bloom: preparation for root review

## Status

**Ready for root's source and correctness review. Nothing was executed on a GPU.**

This session was preparation only. Nothing was built with Cargo. No target, GPU
process, window, render or capture was used, and no grant exists. **I make no
efficacy, appearance or visual-success claim.**

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
  **max**, as configured and verified by root before this prompt. I cannot
  inspect the effort setting from inside the session.
- **Priority continuation applied:** I resumed from the earlier source survey
  instead of restarting it, under the director's bypassPermissions mode.
- **The four known runner mistakes are handled:**
  - Porcelain `-z` output is parsed without stripping.
  - The actual bytes of dirty source files, helper files and compiler inputs
    are frozen.
  - The frame reservation is made under the same lock as the launch.
  - Every exit path finally kills and waits for the owned process group
    before the lock is released.
- **Not touched:** root's uncommitted `brief.md` edit. The runner records its
  actual bytes in every freeze. Nothing was written to memory or outside notes.

**Blocker for root (B1):** one existing test,
`sprites::wide::wide_magnified_nearest_regions_preserve_their_leading_and_trailing_texels`,
statically renders **≈6,264 frames** on its own. That is three times the
2,048-frame ceiling for existing regression work. As the brief requires, the
plan **defers** it, and the runner refuses to run it until root revises the
packet. The other 89 existing tests fit within 1,828 frames.

## Identities to pin in the grants

| Item | Value |
|---|---|
| HEAD / tree | `e263991940b0973ff64f1f5323813d07c6206079` / `b82f5d4d57b6c3084d314698d2b697d83bbbe883` (this result commit adds only `result.md`; re-read HEAD before granting) |
| Engine inputs | Byte-identical to the reviewed union `b633f92d`: `git diff b633f92d HEAD` over `crates Cargo.toml Cargo.lock rust-toolchain.toml .cargo third_party tools/cargo` is empty |
| Helper digest | `58b562262345deeff431b9f2642ab564ad3898f465572c0ca728c706c2fdf84d` (18 files under `tools/`; the build-generated `effects/Cargo.lock` is excluded) |
| `tools/plan.json` sha256 | `dddd15f0daa2f8ee695a52b7173a1fb43e5aa35576776a9d4f10630ef448d38a` (regenerated exactly by `make_plan.py --check`) |
| Repository `.cargo/config.toml` | `0d3b42ee7bc4ef237cb8af440c47e506dcc9cc8b24d49d3cd3d3fea76ae5caa1` (only the Windows MSVC `rustflags`/`rustdocflags` table, which does not apply on macOS) |
| Toolchain | `rust-toolchain.toml` channel `1.99.0`; the build refuses any other `rustc` release |

## Frozen plan (`tools/plan.json`)

**Listing.** `listing.py` predicts the ignored GPU tests from source alone.

- It walks `mod`/`#[path]`/inline modules and default macOS cfg gates.
- It finds 37 lib, 50 model_gpu, 5 game and 2 UI tests: **94 in total**,
  exactly root's union count.
- Before any suite runs, the runner compares each frozen binary's actual
  `--list --ignored` output with this prediction. Any difference refuses that
  suite.

**Budgets:**

| Budget | Planned | Ceiling |
|---|---|---|
| Existing tests (89, static per-test counts with notes) | 1,828 frames, 220 unallocated | 2,048 |
| New ignored tests run inside the lib suite (listed below) | 20, charged to the new budget | — |
| New numerical cases (5) | 73 | — |
| Appearance renders (16 PNGs, 11 matched pairs, ≤960×540) | 16 | — |
| Retry reserve (only with a root receipt) | 19 | — |
| **New frames total** | **128** | 128 |
| Child launches: 4 listings + 4 suites + 5 cases + 1 appearance + 2 retries | **16** | 16 |
| Watchdogs (sum 2,370 s), measured from the first child, compilation excluded | 39.5 min | 45 min |

The four new lib tests and their frames:

- `ambient_occlusion::device_tests::explicit_opt_in_zero_strength_resize_queued_frames_and_failure_preserve_default_output`
  (7);
- `bloom::tests::device::actual_pyramid_nonzero_halo_resize_and_zero_alpha_control`
  (6);
- `bloom::tests::device::canonical_graph_zero_gain_and_pixel_camera_bypass_and_invalid_replace`
  (4);
- `bloom::tests::device::bloom_output_keeps_emitter_position_at_reduced_internal_resolution`
  (3), the reduced-resolution emitter-position test, run exactly once.

**Counting rule.** One charged frame is any of these, including refused
attempts:

- a scene-graph encode, display/output pass, text or UI pass, or compute
  cluster grid;
- a retained submit;
- a composed game tick.

Readbacks are not frames. Existing-test numbers are static counts from reading
the source; root should spot-check the large ones:

- `followed_grid_geometry…`: 780 frames.
- `ties`: 71 + 49 frames.
- `flipbook`: 67 frames.
- `extended_cluster…`: 83, counting 75 compute-only grid encodes as frames.

### Numerical cases

All five run in `tools/effects` and use public APIs only:

- cooked glTF → `SceneInput` → `prepare` or `prepare_retained`, then
  `draw_scene` or `submit`;
- 321×193 targets with the offset fractional viewport `[13.25, 17.5, 280.5, 160.5]`;
- a uniform linear environment (179/255).

Every frame is charged before it is encoded and kept as raw RGBA plus metadata.

**`ssao_core` (20 frames).** A receiver at z = −4 and a slab at z = −3,
x 0.4–3.0, matching the CPU reference.

- In both projections:
  - **white on/off** pairs show the real diffuse occlusion, which may only
    darken;
  - **black (diffuse factor 0)** on/off pairs must be identical;
  - the **isolated flat** control must be identical with AO on and off.
- In perspective only:
  - the **reversed normal sign**, the **far slab** (beyond the radius) and
    **radius 0.5** must each be identical to the off frame;
  - **strength 0.5** must lie between off and strength 1;
  - a **slab crossing the frustum edge** must still occlude on-screen
    receivers; offline it shows the single-layer screen-space limitation.

**`ssao_materials` (10 frames).** AO on and off must be identical for:

- direct light only (environment intensity 0);
- emissive only;
- BLEND only;
- a BLEND slab as the would-be occluder.

A further pair draws opaque, then a BLEND pane, in one pass. It checks the
pipeline and bind-group switch under a validation scope; offline, the change
under the pane must be about (1 − α) of the receiver's change.

**`ssao_geometry` (17 frames).**

- A MASK slab with alpha equal to the cutoff must match the opaque slab
  exactly.
- A MASK slab below the cutoff must be identical with AO on and off.
- A textured MASK cutout is checked against the reference.
- Also covered: a double-sided back-face receiver and a mirrored receiver.
- A **nonuniform + mirrored tilted plane** is a flat control: AO on and off must
  be identical. A wrong normal space would self-occlude.
- **Retained skinned slab:** the t = 0 and t = 1 poses must equal exact baked
  far/near oracles, so a stale or rest-pose prepass would be caught.
- **Retained instance motion:** a far frame, then a near frame that must equal
  the fresh frame.

**`lifecycle_drs` (18 frames).**

- Three frames at the odd sizes 321×193 (A) and 255×147 (B) are encoded in the
  order A, B, A2, then submitted in the order A2, B, A.
- Before submission, the scene, the loaded assets and the source/cache files
  are all dropped.
- An invalid viewport refuses, and so do invalid AO and bloom replacements.
- Each queued frame must equal its in-order reference.
- **SSAO under DRS:** disabled equals fixed 100%; receipts must report
  16/12/8 with the expected internal size; the 75% and 50% frames must equal
  the nearest expansion of internal-size references.
- **Bloom under DRS 16/12/8:** every scale must add a halo and leave the chrome
  unchanged. Offline, the halo centroid must stay on the physical emitter.

**`bloom_numeric` (8 frames).** Three groups, each compared with its own off
frame (2 + 2 + 4 frames):

- **Coverage:** one frame holds BLEND emitters with α = 0 (no change at all),
  0.5 and 1. The halo energy ratio must be 0.4–0.6.
- **Threshold:** one frame holds emitters below, at and above it.
- **Constant field** at 1, 4 and 6 levels: the interior must match the
  tone-curve prediction, 248 vs 243 off, within ±1.
- Chrome must be unchanged throughout.

### Appearance set (16 renders)

Rendered through the canonical composed host with 0067's fonts, UI and script.

- **Review scene:** contact creases, a smooth sphere, a MASK card, opaque and
  BLEND emissive surfaces, a sprite and world text, with a key light of 6.
- **Views:**

  | View | Projection | Size | Viewport and UI | Variants |
  |---|---|---|---|---|
  | v1 | perspective | 959×539 | offset rounded viewport, UI | off, SSAO, bloom, both |
  | v2 | orthographic | 960×540 | full target, no UI | off, SSAO, bloom, both |
  | v3 | perspective (v1 framing) | 959×539 | offset rounded viewport, UI | off and both at DRS 12/16 and 8/16 |
  | v4 | perspective close-up of the creases | 640×480 | full target, no UI | off, SSAO, bloom, both |

- **In-run invariants:**
  - chrome, UI layout boxes and opaque UI interiors are identical to off;
  - the DRS receipts are correct.
- **My appearance verdict comes after the run.** It will cover halos,
  artifacts, normal precision and readability.

## Runner and helper design (for correctness review)

`tools/runner.py` uses only the Python standard library.

- **Separate grants.** The `build` and GPU grants are distinct JSON files that
  live directly in the root-assigned evidence directory. That directory must be
  under `~/.codex/incant-evidence` and outside the worktree. Both grants pin
  `plan_sha256` and `helper_digest`; the GPU grant also pins
  `build_record_sha256`. An absent grant refuses immediately. The runner never
  polls for one.
- **Checks before every child:**
  - HEAD equals the granted commit;
  - engine inputs are unchanged from `b633f92d`;
  - only allowed dirty paths exist: the packet directory outside `tools/` and
    the generated lock, with their actual bytes hashed;
  - the helper digest, frozen binary hashes and helper lock still match.

  Binary hashes are checked again after the child exits.
- **Build step.**
  - The helper `Cargo.lock` is pruned offline and verified as a subset of the
    repository lock.
  - `tools/cargo --locked --offline --profile ci -jN` (N ≤ 2) builds the engine
    test binaries and the helper.
  - **Helper Clippy** runs with `-D warnings`.
  - Binaries are frozen with hashes, and compiler-artifact features are
    recorded. `incant_render` and `incant_render_gpu_host` must have identical
    default features with no `cooked-*` feature.
  - Compiler inputs are hashed: rustup proxies, rustc, cargo, clippy-driver,
    cargo-clippy, rustfmt, sysroot dylibs and std rlibs.
  - A `rustc` release other than the pinned `1.99.0` refuses. A rustup
    directory override would otherwise take precedence over
    `rust-toolchain.toml`.
  - Every Cargo config file Cargo would read is audited by hash and key names
    only: the worktree and its ancestors, plus `CARGO_HOME`. Values are never
    recorded and `credentials.toml` is never opened. Unreviewed files refuse.
  - After compiling, the source, helper bytes, compiler inputs and Cargo config
    audit are all checked again. Any change refuses, and no build record is
    written.
- **Environment.** It is built explicitly. Receipts record key names and
  approved values only.
  - **Build children** get `PATH` (the rustup proxies plus system paths),
    `CARGO_TARGET_DIR`, a private `TMPDIR`, `CARGO_TERM_COLOR`,
    `CARGO_INCREMENTAL=0` and `LC_ALL=C`. HOME, CARGO_HOME and RUSTUP_HOME are
    forwarded unchanged when set, and recorded without their values.
  - **GPU children** get only the system `PATH`, a private `TMPDIR`,
    `RUST_BACKTRACE=1`, `LC_ALL=C` and the granted `INCANT_EXPECT_GPU_BACKEND`.
    They get no HOME.
  - **Refused if present:** any other `INCANT_*` variable, `WGPU_*`,
    `TRACY_*`, `CARGO_{BUILD,FEATURE,PROFILE,ENCODED}_*`, any
    `CARGO_TARGET_*` other than `CARGO_TARGET_DIR`, `RUSTFLAGS`,
    `RUSTDOCFLAGS`, `RUSTC`, the rustc wrappers and `RUSTUP_TOOLCHAIN`.
    Account variables cannot enter, because nothing else is inherited.
- **Child lifecycle.** All of the following happens under one exclusive,
  non-waiting lock:
  - an fsynced reservation;
  - spawn in a new session, with signals blocked across the spawn;
  - a durable `spawned` PID/PGID record;
  - a watchdog;
  - **finally**: SIGTERM, a grace period, SIGKILL to the whole group, a
    blocking reap, and a wait until the group is absent;
  - a completion record written even after a postprocessing error;
  - then the lock is released.

  After the leader is reaped, the group is signalled again only if its members
  started after the spawn, which guards against PGID reuse. Reservations are
  never released.
- **Runner SIGKILL (the documented limitation).** It is uncatchable. The
  reservation stays without a completion, and every later command refuses
  until `reconcile` kills the recorded group by verified identity. `release`
  refuses while any owned group or frozen binary process is alive.
  `release-build` gives a separate receipt for the build lease.
- **The helper itself.**
  - It charges every attempt before encoding and refuses past its declaration.
  - Each renderer gets its own validation scope, which is always popped.
  - It checks that no pipeline is created after setup (wgpu registry).
  - It never enables an effect by default and never calls setup per frame.

## Missing observation capabilities and limitations

- **The raw SSAO attachment cannot be read through public APIs.** It is
  `TEXTURE_BINDING`-only and crate-private. Accessibility is therefore inferred
  from real material-path readbacks: on = D·AO + S, with S from the
  diffuse-factor-0 frame, using interval arithmetic over ±0.75 code, R8 and f16
  quantization. The result is compared with an independent f64 reference whose
  decision margins mark ambiguous rays.
  - "Background 1" inside the attachment is not observable. Only its
    consequence is checked: background pixels are unchanged in the flat
    control.
  - A `cfg(test)` observer would let Astra read the attachment directly. It is
    optional; I have not filed it.
- The inference assumes a uniform-furnace off frame. Uniformity is checked and
  reported as a precondition, and comparisons across children require byte
  identity.
- **Emission is capped at 1** (the glTF importer rejects `emissiveFactor` > 1).
  The bloom numerics therefore use thresholds of 0.25 and 0.5 with exact low
  emission values, and the appearance HDR comes from lighting.
- **Typed refusal of SSAO targets above 4,194,304 pixels is not planned.** It
  needs a target larger than 960×540, so only the CPU `target_bytes` tests
  cover it. This needs a root decision (B3).
- **The Rust helper has not been compiled.** No Cargo was allowed. `rustfmt`
  parsed and format-checked every module. Its first compile and Clippy will be
  root's build step; a compile error there means a helper fix and a new digest
  and grant.
- **The runner's `build` path has never run.** The selftest drives the real
  runner commands with stand-in binaries and a synthetic build record. It
  covers the compiler-input hashing logic only with stand-in tools.
  - The Cargo steps, binary freezing and feature extraction run for the first
    time under the build grant.
  - So does the Cargo config audit: I did not read any file under HOME.
- I used **profile `ci`** throughout. The packet's original step-1 command used
  the dev profile; the test is the same.
- Only Metal is expected on this Mac: no Vulkan, DX12, phone or performance
  claim.
- Out of scope and still pending:
  - temporal upscaling and MetalFX;
  - GPU timestamps (not in this union);
  - cooked SSAO and bloom artifacts (the `cooked-materials` and `cooked-output`
    refusals stay in place);
  - native default exposure, profiling subpass entries and device budgets;
  - complete Phase 1 rendering.
- **Appearance inputs are review content, not presets.** AO radius 2.0 is the
  packet's fixture value, which is large for a 1 m scene. No tuning was done.

## Checks run this session (CPU only, at `e2639919`)

Commands run from the worktree root. `python3` means
`/opt/homebrew/bin/python3 -I -B`, and `tools/` means
`handoffs/0070-ambient-occlusion/tools/`.

| Command | Result |
|---|---|
| `python3 tools/listing.py` | lib 127 tests / 37 ignored; model_gpu 51 / 50; gpu_host-game 6 / 5; gpu_host-ui 3 / 2; **94 ignored** |
| `python3 tools/make_plan.py --check` | `plan.json matches the source-derived plan` |
| `python3 tools/selftest.py` | **18 of 18 controls pass**, exit 0 (details below) |
| `python3 tools/ssao_reference.py --selftest` | Matches the engine CPU test's claims in both projections: flat [0, 0] rays, near slab [5, 5] (AO 11/16), reversed normal [0, 0]; also asserts strength-0.5 linearity |
| `python3 tools/analyze.py --selftest` | Synthetic correct readbacks: 43,489 interior pixels agree in each projection with no disagreement (8,558 perspective and 7,075 orthographic occluded). A +2-ray error injected on every 97th diagonal is detected: 457 disagreements per projection |
| `/opt/homebrew/opt/rustup/bin/rustfmt --edition 2024 --check tools/effects/src/lib.rs tools/effects/src/main.rs` | Exit 0 (no Cargo) |

The 18 controls cover:

- porcelain parsing, where a stripped stream is refused;
- environment allowlists;
- **new:** compiler inputs with stand-in tools — the record is stable when
  nothing changes, changes when a tool is swapped, and an unpinned release
  refuses;
- ledger ceilings;
- the non-waiting lock;
- normal exit, timeout, SIGTERM-ignored escalation and grandchild kill;
- a failed `on_spawn` write;
- SIGINT and SIGTERM to a runner mid-wait;
- the SIGKILL orphan and its recovery;
- grant refusals;
- end-to-end runner flows with stand-in binaries: listing before suite,
  receipts, a timeout, malformed receipts, duplicate and retry refusals, and
  runner SIGKILL → reconcile → release → post-release refusal.

**Failures and corrections:**

- The first full selftest run failed one control because of my test's
  expectation, not the runner. The duplicate was refused by the output-guard
  rather than the label-guard. I fixed the assertion (`9a640a2b`); the run is
  superseded.
- The reference's first self-test exposed a flaw in its own ambiguity logic. I
  fixed it before any commit.
- I wrote one `rustfmt` check output to `/tmp`, outside the worktree, and
  deleted it immediately.
- No CPU-control processes or scratch directories remain (checked with `ps`
  and a directory listing).

## Proposed check list for root

1. **CPU, no grant:** the six commands above, plus
   `python3 -m py_compile` on the helper scripts if wanted.
2. **Under a build grant** (`incant-0070-build-grant-v1`):
   `runner.py build --grant <evidence>/build-grant.json` (this includes helper
   Clippy, the toolchain pin and the feature identity), then
   `runner.py release-build`.
3. **Under a GPU grant** (`incant-0070-gpu-grant-v1`, pinning
   `build_record_sha256` and `expected_backend: "Metal"`):
   1. `list` four times;
   2. `suite` four times;
   3. `case` five times;
   4. `appearance`;
   5. `release`;
   6. then `analyze.py <evidence>`, followed by my appearance review.

Example GPU grant fields:

```json
{"format": "incant-0070-gpu-grant-v1", "packet": "0070-ambient-occlusion", "kind": "gpu",
 "source_commit": "<HEAD>", "plan_sha256": "dddd15f0…", "helper_digest": "58b56226…",
 "evidence_dir": "/Users/andreyyurlov/.codex/incant-evidence/0070", "build_record_sha256": "<build.json>",
 "expected_backend": "Metal", "allow_deferred_tests": []}
```

The build grant adds `cargo_target_dir`, `max_jobs` (≤ 2) and
`approved_cargo_configs` (`{path: sha256}`) and has no backend field. A retry
needs a root-written `incant-0070-retry-v1` receipt naming the completed
original child and a reason.

## Decisions for root

1. **B1:** revise the packet for the wide sprite sweep (≈6,264 frames), or
   accept its deferral from this regression.
2. **B2:** the plan counts the four `--list` launches inside the 16-child cap.
   Confirm, or exclude them and gain retry slots.
3. **B3:** should a typed-refusal probe above 4,194,304 pixels be allowed? It
   needs a target larger than 960×540, so it is not planned.
4. **B4:** is the Cargo config audit acceptable? It hashes the worktree,
   ancestor and `CARGO_HOME` config files and records key names only. Any file
   other than the reviewed repository config must be listed in
   `approved_cargo_configs`. Or do you prefer another way to reject overrides?

## Changed paths

All are new, under `handoffs/0070-ambient-occlusion/`:

- `result.md`
- `tools/`: `common.py`, `lifecycle.py`, `ledger.py`, `runner.py`,
  `listing.py`, `make_plan.py`, `plan.json`, `selftest.py`,
  `ssao_reference.py`, `analyze.py`
- `tools/effects/`: `Cargo.toml`, `src/{lib,main,common,gltf,numeric,composed,appearance}.rs`

Commits: `0029aa12`, `9a640a2b`, `d11293f1` and `e2639919` (the toolchain pin
and post-build compiler recheck), plus this result, all with
`Built-by: claude`. No engine, repository manifest, CI or root-written file
changed. Nothing was pushed or merged. Screenshots: none (no rendering was
authorized).
