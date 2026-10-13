# 0073 owner_quality: preparation result

**Status:** `PREPARED_FOR_ROOT_REVIEW_NO_GRANT`.

This turn was CPU-only. None of the following happened:
- no Cargo run, compile, lint, renderer, GPU process or capture;
- no image generation and no grant;
- no production edit, push or merge.

No phase gate is approved or claimed. Every historical failure stands.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`), resuming session Claude0073. Root configured Max thinking through ACP before prompting; I cannot re-check that setting from inside the session. Session record: `0073-owner-quality-preparation-001`.

## Inputs

**Brief**
- Committed at b7099550: sha256 `311d12bc086889fe3b478c3b6c9108b41a100c5349953a521e9e645a01ef2d0b`.
- With root steering: `c8529663d0548163ce780b64fdc7b0cd310b7c668f2428ae185d77f55e632056`.
- `ROOT_STEERING.md`, which includes Astra's correctness review: `8cd864b90fe5c74618f66bc82005948f1f51b42182fd2ab3828186e5f64d5c4f`.

**Production**
- `fd5b86d03a2cabfcc9bd5a8441a137645fb858ae` in native-agent-play.
- Freeze `608ad362…`: all 2737 inputs equal their fd5b86d0 blobs.
- This child's older engine tree is never used.

## Changed paths

All paths are new, under `handoffs/0073-upscaling-review/owner_quality/`.

| Path | Role |
|---|---|
| `review/upscaling_owner_review.rs` | Packet-local Rust review fixture: 3076 lines, sha256 `650316a21f30bfb4f7cbe8c99b67f9b5be269b51eb62b4d017b519bd5130d997` |
| `plan.json` | Source identity, build-injection map, run order and environment, exact budgets, API, outcome rules, historical record, limits |
| `tools/check.py` | Deterministic CPU self-check using the standard library and read-only git |
| `preparation/selfcheck.json` | Self-check output |
| `preparation-result.md` | This file |

I did not stage the following, and left them as I found them:
- the dirty parent `brief.md`;
- the root-modified `owner_quality/brief.md`;
- `owner_quality/ROOT_STEERING.md`;
- the three untracked `owner-attachment-*` inputs.

## Fixture

The fixture was adapted from `visual/review/upscaling_review.rs` (`c0169e34…`), which is unchanged. Of the old fixture's shared lines 480-1330, 850 of 851 are verbatim. rustfmt re-laid out one method chain.

**Renderer:** one per group, with the production startup order `enable_temporal_upscaling` then `enable_motion_owner`.

**Variants** share the renderer, the inputs and the chronology:
- **FULL:** unjittered output-size control.
- **DRS:** fixed internal scale.
- **UP:** motion history, then `enable_temporal_upscaling(REVIEW)`.
- **OWN:** motion history, then `set_motion_owner_attachment(true)`, then `enable_temporal_upscaling(REVIEW)`.

The REVIEW policy is unchanged. The physical UI is kept on the stationary group.

**Primary groups**
- **`stationary`**: matched UP and OWN with FULL and DRS controls.
- **`motion`**: the same comparison under object and camera motion.
- **`transitions`**: corrected schedule `8/16×10, 12/16×3, 4/16×3, 16/16×4, 8/16×3, cut, 8/16×5`. The extra 16/16 frame makes the C9 fourth frame measurable.

**Secondary groups**, fixture-only on the existing public API:
- **`c7_disocclusion`**: never-occluded, same-chronology reference.
- **`p1_crossing`**: minimal P1 crossing-panel/bar control with a matched bar-free reference.

## Astra's corrections

These are reporting and validity changes only. Threshold values, renders, budgets and shaders are unchanged.

- **Renamed outputs.** `historical_gates`, `c7_as_written` and `c9_as_written` are now `historical_threshold_observations.{c7,c9}`. Each carries a status saying it is a rendered-output observation on tone-mapped sRGB codes, not a pass or fail of the historical linear-model gate. The "as written" and "converged" claims are gone.
- **Old failures kept.** `plan.json` keeps the old failures under `c7_original_cpu_gate` and `c9_original_cpu_gate`. They still read 0.0918, and 0.683 / 0.869 and 0.741 / 0.943, all failed.
- **C9 reference.** Now `matched_cycle_reference`: the mean of frames 17-24 (one 8-phase jitter cycle) of a separate 24-frame fixed-16/16 run. It is described as a finite matched-cycle reference; convergence is not demonstrated. The `steady16` names are now `fixed16`.
- **C9 ratios.** `contrast_ratio` returns null unless the numerator is finite and the denominator is finite and positive. Comparisons with an undefined ratio are null.
- **C9 all-region summaries are three-valued.** Each lists `undefined_regions`:
  - `false` if any defined region is below the threshold;
  - `null` if none is below but some region is undefined, or there are no regions;
  - `true` only when every region is defined and meets the threshold.
- **C7.** `pass` is now `worst_at_most_0_06`. It is null for an empty column or row range or a nonfinite column.

## Build-injection map (root)

1. Make a checkout that is source-identical to fd5b86d0. Copy `review/upscaling_owner_review.rs` to `crates/incant_render/tests/upscaling_owner_review.rs`, which is a new path at fd5b86d0, and verify its sha256.
   - It uses the existing `tests/materials/mod.rs` and `tests/support/mod.rs` and three `incant_text` test fonts; these blobs are pinned in `plan.json`.
   - Every crate it uses is already an incant_render dependency or dev-dependency, so `Cargo.toml` does not change.
2. Build: `./tools/cargo test --locked --offline --profile ci -j2 -p incant_render --test upscaling_owner_review --no-run --message-format=json-render-diagnostics`. Use default features (the portable WGSL path) and refuse `cooked-*` or `native-metalfx`.
3. Lint: `./tools/cargo clippy --locked --offline --profile ci -j2 -p incant_render --test upscaling_owner_review -- -D warnings`.
4. Run, once a grant exists: one process per group, in the foreground, `<frozen binary> --ignored --exact <group> --nocapture --test-threads=1`.
   - **Environment:** `PATH=/usr/bin:/bin:/usr/sbin:/sbin`, a fresh `TMPDIR`, `INCANT_EXPECT_GPU_BACKEND=Metal`, `RUST_BACKTRACE=0`, and `INCANT_0073_OWNER_REVIEW_OUT=<fresh root>/<group>` (a directory that must not exist yet).
   - **Order:** primary groups first. Stop at the first failure, with no retry.

## Exact bounds

Every scene, UI and readback submission is charged before it is attempted, and each readback has exactly one map. A report that does not consume its budget exactly fails.

| Group | Scenes = frames | UI | Readbacks = maps | Queue | PNGs |
|---|---|---|---|---|---|
| stationary | 50 | 18 | 50 | 118 | 6 |
| motion | 44 | 0 | 44 | 88 | 4 |
| transitions | 106 | 0 | 106 | 212 | 2 |
| c7_disocclusion | 82 | 0 | 82 | 164 | 2 |
| p1_crossing | 98 | 0 | 98 | 196 | 1 |
| **All (5 processes)** | 380 | 18 | 380 | 778 | 15 |
| **Primary only (3 processes)** | 200 | 18 | 200 | 418 | 12 |

- **Images:** 960×540 PNGs, RGBA8 sRGB codes, 2,073,600 raw bytes each, at most 2,100,000 file bytes each.
- **Processes:** 90 s timeout each.
- **Outputs per group:** `events.jsonl`, `report.json` and the group's PNGs only.

## API

**Required:** listed in `plan.json` `api`. `check.py` verified each item as public at fd5b86d0.

**Missing public API:**
- No read of the owner record or of per-pixel persist/reproject/drop decisions.
- No replay of the private drop chronology, which limits P1's reference.
- No jitter-phase selection. UP and OWN stay phase-matched only through identical submission counts.

## Capture selection and appearance criteria

Contract checks decide each process outcome. Numbers never override a visible judgment, and the reverse also holds. Each judgment compares OWN with UP on the same frame, with FULL and DRS as references.

**Main condition:** any OWN regression against UP in a primary PNG blocks an owner appearance claim. The a5ae3179 FAIL and the GPU001 failure stand until a later review records otherwise.

**Stationary**
- `stationary-full`, `-drs50`, `-up50`, `-own50` (frame 24, UI on): overall softness and texture detail against DRS. OWN must not be softer or noisier than UP, and the UI must be identical.
- `stationary-phases`: the wires crop across jitter phases 0-7, UP against OWN. Look for shimmer, breaks and presence of the sub-pixel wires.
- `stationary-detail`: wires, card edge, sign and sprite across FULL, DRS, UP and OWN. Look for edge stability, aliasing and text and sprite clarity.

**Motion**
- `motion-full`, `-up50`, `-own50` (frame 20, camera pan): smear and ghosting on the pan.
- `motion-detail`: the mover at frames 9, 11 and 14, where trailing and ghosting start, and the pan crops. The ×4 error maps against FULL should show OWN error no larger or more structured than UP's.

**Transitions**
- `transitions-full`: the full-frame FULL control.
- `transitions-strip`: pops at each scale change, recovery on the third and fourth 16/16 frames against fixed-16/16 frame 24, and the first frame after the cut.

**Secondary**
- `c7-detail` and `c7-own-moving`: the disocclusion trail behind the square. The difference maps should be dark more than 2 frames behind its trailing edge.
- `p1-detail`: residual from the crossing bar around the panel after it stops, against the bar-free reference.

## CPU checks run this turn

- **Self-check:** `python3 -I -B handoffs/0073-upscaling-review/owner_quality/tools/check.py --out handoffs/0073-upscaling-review/owner_quality/preparation/selfcheck.json`.
  - All 8 checks pass: identity, budgets, schedule, pngs, tests, api, freeze (2737 inputs, 0 missing, 0 mismatched) and static (13 rules).
  - Two runs gave byte-identical output.
  - `/usr/bin/python3` 3.9.6 output is identical apart from the version field.
- **Mutation test of the new static rules:** an ad-hoc run, not persisted, that imported `check.py` and edited the source in memory. Each of these six mutations was caught by its intended rule:
  - dropping the positive-denominator guard;
  - a raw third/reference division;
  - a vacuous all-region true;
  - a C7 `pass` key;
  - a `historical_gates` key;
  - a lowered C7 limit.
- **Formatting:** rustfmt 1.10.0 from the pinned 1.99.0 toolchain, `--edition 2024 --check`, reading the fixture from stdin: exit 0. The formatting changes I adopted were layout plus 29 trailing commas; no tokens changed otherwise.

## Limitations

- **Not tested yet:** the fixture is not compiled or linted. Root's targeted build and clippy are the first compiler check.
- **Coverage:**
  - The default WGSL path only.
  - One local Metal adapter.
  - No phone, performance, memory or phase claim.
- **C7 and C9:** their units on 8-bit tone-mapped output differ from the linear CPU model's. Their values are observations only.
- **P1:** one grey background at 8/16 with one stop. The reference cannot replay the run's own drops.

## Open questions for root

- Whether to run the secondary groups (C7, P1) or only the three primary groups.
- An empty or flat C9 chart region now yields null observations rather than failing the process. If you want that to fail instead, a region-measurability contract check would be a separate outcome-rule change.
