# 0070 SSAO filter execution namespace 0070-ssao-filter-001: prepared helpers and binding

## Status

**Preparation only.** There was no Cargo, build, target, GPU, capture,
production edit, grant, push or merge. No build or GPU authority follows from
this report.

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
  **max**. ACP verification is root's.
- **Builds on:** the accepted cc2e312a observer and case proposal.
- **Unchanged:**
  - `tools/` (build004's released orchestration and helper). Its helper digest
    is still `5ec49552…` over 20 files;
  - root's production source and every grant;
  - all old evidence and the earlier reports, including `result.md` (the
    build002 report);
  - the accepted `selftest-001.json`.
- **Root's source, read but not pinned:**
  - **Committed:** R1/R3 at 9f3a6b03 on `codex/ssao-filter-repair`. It contains
    both named control tests, and the device test returns early with the typed
    `CookedMaterials` refusal under `cooked-materials`.
  - **Pending:** the `ssao-stage-readback` feature, which is not in
    `incant_render`'s Cargo features yet.
  - **Unresolved identity:** `d6b84b43` is not an object of this repository;
    I record it as root's review identity.

## What is implemented

Everything lives in the new namespace `handoffs/0070-ambient-occlusion/ssao-filter-001/`:

| Path | Purpose |
|---|---|
| `effects/` | A copy of build004's fixture crate with only the documented changes (below), and its own package name `incant_0070_ssao_filter_effects` |
| `filter_common.py` | The namespace binding, reusing `tools/common.py` by import. It overrides the plan, helper, frozen engine commit, grant formats, digest and freeze scope |
| `filter_runner.py` | Reuses `tools/runner.py`, `lifecycle.py` and `ledger.py` unchanged. Adds the build with feature binding, the exact controls, stage-aware output validation and an ordering guard. It has no retry command |
| `make_plan.py`, `plan.json` | The execution plan, re-derivable with `--check`. Its cases come from the accepted `cases.json`, and its appearance set is build004's |
| `selftest.py` | Focused CPU guards for the namespace binding |

**Helper changes, all checked mechanically by `selftest.py`:**

- **Unchanged:** `appearance.rs`, `composed.rs` and `gltf.rs` are identical.
- **`numeric.rs` (only insertions plus documented edits):** build004's file
  survives as an in-order subsequence of the copy, and exactly 12 call sites
  are renamed.
  - **New methods:** `Run::draw_with_stages` and `Run::submit_with_stages`.
    Each arms the one-shot capture, makes one charged draw or retained submit,
    then takes the planes. Planes that are missing, the wrong size or unarmed
    refuse the case.
  - **Stage frames:** the 12 renamed call sites produce 13 stage frames, since
    the core loop renders both projections.
  - **New case:** `ssao_quality`, with 10 frames and 2 more stage frames
    (stripes, both projections).
  - **New in-run checks:**
    - the sphere is identical with and without AO;
    - the sphere covers its analytic silhouette within 10% (2,824.6 px
      perspective, 455.2 px orthographic);
    - the stripes darken and never brighten.
- **`common.rs`:** `Store::keep_stages` writes `<label>.{raw,h,v}.r8` with
  create-new, an exact length check and `r8_sha256` index entries.
- **`Cargo.toml`, `main.rs`, `lib.rs`:**
  - the package and library are renamed;
  - `incant_render` is pinned to `features = ["ssao-stage-readback"]`.

**Analyzer additions** (`ssao-repair/filter/`, focused only):

- **Runtime-measured R8 rounding:** at every AO-pass texel whose value is
  exact, the stage gate records the device code minus `255 × value` and which
  way exact ties went. This is an observation, not a tolerance; the code must
  still lie in its round-to-nearest interval.
- **Run binding:** the report records the build record's namespace, plan,
  engine, features and frozen binaries. Numeric pass requires that the plan
  analyzed is the plan that was built.
- **Thresholds:** unchanged, with no blanket slack added. The two new controls
  are recorded in `selftest-002.json`.

## API this helper calls (root implements before freeze)

```rust
renderer.arm_ambient_occlusion_stage_capture() -> incant_render::Result<()>
renderer.take_ambient_occlusion_stages() -> incant_render::Result<AmbientOcclusionStages>
// AmbientOcclusionStages { size: [u32; 2], raw: Vec<u8>, horizontal: Vec<u8>, vertical: Vec<u8> }
```

- **Planes:** tightly packed, at the internal size.
- **Coverage:** the capture must work through both `draw_scene` and
  `RetainedScene::submit`. Four geometry stage frames (`skin_t0`, `skin_t1`,
  `motion_far`, `motion_near`) use the retained route.

## Source binding and freeze

These are exact steps for root:

1. **Land the readback commit** on `codex/ssao-filter-repair`, providing the
   feature and API above.
2. **Cherry-pick the frozen engine commits onto this branch** after this
   preparation commit: 72a16a9d, 9f3a6b03 and the readback commit. HEAD's
   engine inputs (`crates`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
   `.cargo`, `third_party`, `tools/cargo`) must then equal root's final engine
   commit byte for byte.
3. **Bind the engine commit:**

   ```
   python3 -I -B handoffs/0070-ambient-occlusion/ssao-filter-001/make_plan.py --engine-commit <root's final 40-hex commit>
   ```

   This verifies that the commit exists, contains both control test functions,
   and declares `ssao-stage-readback`. Then commit `plan.json` and rerun
   `make_plan.py --check` and `selftest.py`.

   Until this step, `plan.json` has `engine_commit: null`, and every build or
   GPU command refuses.
4. **Build grant:** write
   `/Users/andreyyurlov/.codex/incant-evidence/0070-ssao-filter-001/build-grant.json`
   with format `incant-0070-ssao-filter-001-build-grant-v1` and these fields:
   - `packet` (`0070-ambient-occlusion`), `namespace`, `kind` (`build`);
   - `source_commit` (HEAD), `engine_commit`;
   - `plan_sha256`, `helper_digest` (as printed by `selftest.py`);
   - `evidence_dir` (that directory), `cargo_target_dir`, `max_jobs` (2 at
     most), `approved_cargo_configs`.
5. **GPU grant:** after the build, a matching `gpu-grant.json` with format
   `incant-0070-ssao-filter-001-gpu-grant-v1`, plus `build_record_sha256` and
   `expected_backend`.

A grant whose evidence directory is not exactly `…/0070-ssao-filter-001`, or
that uses build004's grant format, refuses.

## Features (exact, bound by the build record)

| Binary | Cargo | Features required |
|---|---|---|
| `lib` | `test --no-run -p incant_render --lib --features ssao-stage-readback` | Exactly `default, physics-deterministic, ssao-stage-readback` |
| `effects_gpu` | `build --manifest-path ssao-filter-001/effects/Cargo.toml --bin effects_gpu`, plus clippy with `-D warnings` | Its `incant_render` unit equals `lib`'s exactly; no `cooked-*` |
| `lib-cooked` | `test --no-run -p incant_render --lib --features cooked-materials,ssao-stage-readback` | Contains `cooked-materials` and `ssao-stage-readback` |

- **Invocation:** every Cargo call uses build004's flags: `--locked --offline
  --profile ci -j<jobs> --message-format=json-render-diagnostics`, the explicit
  environment and audited Cargo configs.
- **If the actual set differs** (for example, if `ssao-stage-readback` implies
  another feature), the build refuses and records it. The expected list in
  `make_plan.py` is then a reviewed change, never silent acceptance.

## Commands (after root's grants)

Run in this order from the worktree root, each under
`env -i PATH=/usr/bin:/bin LC_ALL=C /opt/homebrew/bin/python3 -I -B`.
Here `R` is `handoffs/0070-ambient-occlusion/ssao-filter-001/filter_runner.py`
and `E` is `/Users/andreyyurlov/.codex/incant-evidence/0070-ssao-filter-001`.

```
R build          --grant E/build-grant.json
R release-build  --grant E/build-grant.json
R control        --grant E/gpu-grant.json filter_device      # 2 submissions, 6 passes, no images
R control        --grant E/gpu-grant.json ao_device          # 7 submissions, no images
R control        --grant E/gpu-grant.json ao_device_cooked   # 0 submissions: typed refusal (optional)
R case           --grant E/gpu-grant.json ssao_core          # 20
R case           --grant E/gpu-grant.json ssao_materials     # 10
R case           --grant E/gpu-grant.json ssao_geometry      # 17
R case           --grant E/gpu-grant.json lifecycle_drs      # 18, DRS bloom centroid kept
R case           --grant E/gpu-grant.json ssao_quality       # 10
R appearance     --grant E/gpu-grant.json                    # 16 renders, 16 PNGs, 11 pairs
R release        --grant E/gpu-grant.json                    # always, including after any failure
handoffs/0070-ambient-occlusion/ssao-repair/filter/analyze_filter.py E   # after release only
```

Each control runs the frozen binary as
`--ignored --exact --test-threads=1 <test>`, with libtest's exact count and
summary checked. The full lib suite is not repeated, and `bloom_numeric` is
not run.

## Budgets

| Item | Planned | Ceiling |
|---|---|---|
| GPU children | 9 (3 controls, 5 cases, appearance) | 9 |
| New submissions | 100 = 2 + 7 + 0 + 75 + 16 | 128 (28 unallocated) |
| Existing-regression frames | 0 | 0 |
| PNGs / pairs | 16 / 11, at most 960×540 | 32 / 16 |
| Watchdogs | 1,290 s | 2,700 s execution ceiling |
| Retries | none (no retry command, 0 reserved) | — |

## Failure semantics

**Device work stops on any child failure.** The triggers are:

- a nonzero exit, a signal or a timeout;
- an unverified process group, or a changed binary;
- a libtest mismatch;
- a missing or failed helper verdict;
- an invalid index, or stage planes that are missing or malformed.

Any of these latches the namespace. Every later device command refuses; root
releases with `release` and makes no automatic retry. Controls are recorded as
exact `suite` children so that the reused ledger latches them; `selftest.py`
proves this.

**Ordering is enforced:**

- cases refuse until both required controls have succeeded;
- appearance refuses until all five cases have succeeded.

**Quality never stops device work.** It is analyzed only after release, and
`analyze_filter.py` records it honestly. Any GPU result that agrees with the
oracle must reach these verdicts (oracle summary):

| Quality gate | Prediction |
|---|---|
| N6 (authored) | **Fails** (7–8 codes) |
| N8 perspective | **Fails** (0.376–0.439) |
| N7 | Passes |
| N8 orthographic | Passes |

The analyzer then exits 1 with its report retained. That is failed quality,
not a device failure. N6T stays a diagnostic.

## Evidence from this preparation (CPU only)

| Command | Result |
|---|---|
| `make_plan.py --check` | `plan.json` reproduces exactly; 100 of 128 new submissions; 9 children |
| `selftest.py` | Exit 0 in 0.15 s (details below) |
| `analyze_filter.py --selftest` → `selftest-002.json` | Exit 0 in 76 s (details below) |
| `tools/` helper digest | `5ec49552…` with 20 files, unchanged |
| Namespace helper digest | `2754cb57bd045abda20a32528b4122cad03672b73aee09ddb3969734725728db` over 26 files |

**`selftest.py` covers:**

- refusal while the engine commit is unfrozen;
- grant refusals for build004's format, a wrong plan, engine commit or digest,
  and a non-namespace evidence directory;
- stage-plane index refusals for a missing plane, an undeclared frame, a short
  plane, a declared frame with no planes and a tampered plane;
- feature-binding refusals for an absent readback feature, a lib/helper
  mismatch, a cooked family in the default build, and a cooked control without
  `cooked-materials`;
- ordering, and latching of a failed control;
- the helper diff scope;
- digest coverage: 6 reused tool files, 7 analyzer files and the namespace;
- the CLI: refusing without a grant exits 2.

**`analyze_filter.py --selftest` (`selftest-002.json`):**

- everything accepted in cc2e312a still holds;
- the rounding measurement sees about 45,000 exact texels per fixture, and all
  415 `white_slab` ties go up, as the round-half-up model predicts;
- a build record from another plan makes numeric pass false.

## Remaining gates (candid)

- **Numeric, pending the run:**
  - device agreement and the 15 staged R8 checks;
  - identity, including the sphere;
  - the DRS and determinism gates on the filter build;
  - the measured device R8 rounding.
- **Quality:** N6 and N8 perspective are predicted to fail and stay failed; no
  gate is waived.
- **Appearance:** my rendered-pair review against the build003 crops.
- **Not authorized:** cooked admission, default enablement, art tuning, and
  device budgets or phone measurements.
- **Build004's own analysis** is root's to report; I did not change it.

## Return record

**Changed paths:**

- new: `ssao-filter-001/` (13 files) and `ssao-filter-001-preparation.md`;
- modified: `ssao-repair/filter/analyze_filter.py` and
  `ssao-repair/filter/controls.py`;
- new evidence: `ssao-repair/filter/selftest-002.json`.

**Screenshots:** none.

**Limitations:**

- The Rust is uncompiled, because no Cargo was authorized. It calls the
  readback API exactly as proposed, so a different shape will fail compilation
  at root's build.
- The engine commit is not frozen.
- Retained-route capture depends on root's implementation.

**Open questions for root:**

1. Does the readback API match the shape above, including `RetainedScene::submit`?
2. Does `ssao-stage-readback` imply any other feature?
3. Should the optional cooked control run (0 submissions)?
