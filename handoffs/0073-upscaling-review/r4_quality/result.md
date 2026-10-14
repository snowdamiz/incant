# 0073 R4 appearance preparation: result

**Status: PREPARED, NOT EXECUTED.** The packet is ready for a later root build and grant. Nothing was built, run on a GPU, captured or rendered, and no R4 pixel has been reviewed. Still pending:
- a fresh isolated build of candidate 825ff with fixture 650316a;
- a root grant;
- the bounded execution;
- my pixel review.

This turn passes no gate and accepts no appearance. All earlier quality failures remain open: overall reconstruction rejected, texture/fine detail, phase-dependent thin geometry, silhouettes, the DRS wipe, C7, C9, P1, `visual_a5ae3179` and GPU001 case 3.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max; per the packet, root verified both through ACP. Auto-memory is off; no memory or preferences were read or written.

**Turn bound:** 15 minutes, started 21:48:28 local (monotonic 122744.87).

## Immutable inputs

| Input | Identity |
|---|---|
| Candidate build source | `825ff047284e84252d00f786a7b14cb6ecb44293` (checkout HEAD observed read-only: same) |
| Production equivalent | `72b1ba8ec351e86d66a90c401e0a75a530633616` (test-only correction `cae2bffc…`) |
| `r4_quality/production-freeze.json` (root-provided, read-only) | sha256 `9e36e8c3a8d7253a3619a85f8cb8e3e313c663bb37c9f15dab5946528c164f62`; 2839 files; `source_commit` 825ff; `production_equivalent` 72b1 |
| Fixture `upscaling_owner_review.rs` | sha256 `650316a21f30bfb4f7cbe8c99b67f9b5be269b51eb62b4d017b519bd5130d997`, reused byte for byte in `review/` |
| Quality plan `owner_quality/plan.json` | `ff0f0f488faa3f6d4d6cc1321d5c4e87c35cbb4160130dddd050697bb824dbc4` (unchanged) |
| Lifecycle `owner/owner_runner.py` | `b9e12895389a046e1c0a0bd4c43f01a215505c793be4106b31fafc18aa2aff34` (unchanged) |
| Previous execution runner (76be) | `owner_output_quality/execution/runner.py` |

## 1. Compatibility (`review/compatibility.json`, `execution/selfcheck.json`)

**Verdict:** compatible by source inspection. The fixture is reused unchanged and needs no adaptation. It has not been compiled.

**Evidence:**
- **Accessors the fixture uses:** `MotionFrame::jitter_phase() -> Option<u32>` (fixture lines 523, 568 and 601) and every other `MotionFrame` accessor the fixture calls are present at 825ff. The jitter schedule still has eight phases.
- **Changed public functions are unused:** `Prepared::jitter_phase` became `jitter_offset`, plus the evidence and kernel test oracles. The fixture uses none of them.
- **Unchanged from 76be (the previous run's base) to 825ff:**
  - `upscaling/mod.rs` (`UpscaleConfig`, `UpscaleFrame`);
  - `resolution/`;
  - `incant_render/Cargo.toml` and its `tests/`;
  - the workspace `Cargo.toml` and `Cargo.lock`;
  - `incant_assets`, `incant_doc`, `incant_text`, `incant_render_data`, `incant_render_authoring` and `incant_types`.
- **Behaviour changes affect pixels only:** the estimator, LOD bias, split jitter offsets and DRS guard change pixels. They do not change the API, phase indices, history, epochs or internal sizes that the fixture asserts.

## 2. Preparation plan (`execution/`)

- **`runner.py`** (sha256 `2a991b3abde5fb2b5e5ddcc28eac6187f88e0f5fa4b1c7dbe235eae112b8af6c`) is the 76be execution runner with only source, path and hash metadata changed (`protocol-delta.patch`):
  - the fixture path points to `r4_quality/review/`;
  - `PRODUCTION` is 72b1 and a new `SOURCE` is 825ff;
  - the freeze sha is updated;
  - the evidence directory is `0073-r4-quality-gpu-001`;
  - the build receipt must name `SOURCE`;
  - the verified-file count is 2839, matching the freeze.

  No assertion, budget, environment, lifecycle or artifact check changed.
- **`plan.json`** (sha256 `e019cd7fb3ea6a2bf4315125a2c0806650a6141cc82e4e00237e6c44cc39b760`) is in state `AWAITING_ROOT_BINARY`, with build, enumeration, build-review and binary all `null`. No grant, build or binary is invented, and an unbuilt plan cannot execute.
- **`grant-template.json`** is a template only: root fills in the issue time, the frozen plan sha and the binary sha. A fresh root grant is required.

## 3. Exact maximums and per-group counts

| Group (primary-first order) | Scenes | UI | Readbacks | Maps | Queue submissions | PNGs |
|---|---|---|---|---|---|---|
| stationary | 50 | 18 | 50 | 50 | 118 | 6 |
| motion | 44 | 0 | 44 | 44 | 88 | 4 |
| transitions | 106 | 0 | 106 | 106 | 212 | 2 |
| c7_disocclusion | 82 | 0 | 82 | 82 | 164 | 2 |
| p1_crossing | 98 | 0 | 98 | 98 | 196 | 1 |
| **Total** | **380** | **18** | **380** | **380** | **778** | **15** |

Further limits: 5 child processes, PNGs at most 960×540, 90 s per child, stop at the first failure, and no retries, extra probes, images or builds.

**CPU-only self-check:** `execution/selfcheck.py` → `selfcheck.json`, 17/17 pass. It never takes the run branch. It verifies:
- the fixture identity;
- the runner diff, which must equal exactly the six metadata substitutions;
- the lifecycle sha;
- the plan through the runner's own `read_plan`;
- the exact maximums and five-group order;
- the freeze binding;
- the API pins.

Its fail-closed checks all refuse:
- an unbuilt plan;
- a grant without a frozen binary;
- a mutated frozen state without receipts;
- a 16th PNG;
- the old producer;
- a retry;
- a dropped group.

The only runner invocation was `runner.py check`, which reported "grant absent; no GPU authority".

## 4. Pixel review

`review-checklist.md` covers the later granted run:
- all 15 PNGs;
- FULL, DRS, ordinary UP and OWN, compared for usable reduced-resolution quality rather than identity with FULL;
- detail, thin features, overlays and stability judged together;
- the C7, C9 and P1 thresholds quoted unchanged and kept separate from the sRGB and finite-reference observations.

There are no new thresholds, and failure is an allowed outcome.

## Changed paths and commands

**New files (only these are committed):**
- `r4_quality/result.md`
- `r4_quality/review-checklist.md`
- `r4_quality/review/upscaling_owner_review.rs` (copy) and `review/compatibility.json`
- `r4_quality/execution/runner.py`, `plan.json`, `grant-template.json`, `protocol-delta.patch`, `selfcheck.py` and `selfcheck.json`

**Read-only and not committed:** root's `r4_quality/brief.md`, `inputs/` and `production-freeze.json`. The brief, every old packet, the top-level `result.md` and all untracked files are preserved.

```sh
cd handoffs/0073-upscaling-review/r4_quality/execution
python3 -I -B runner.py check "$PWD/plan.json"   # AWAITING_ROOT_BINARY; grant absent; no GPU authority
python3 -I -B selfcheck.py --checkout /Users/andreyyurlov/.codex/worktrees/owner-quality-repair/incant   # checks=17 failed=0
```

**Scratch:** `/tmp/incant-0073-r4-quality-*` (mutated plans), deleted automatically.

## Missing evidence: explicitly pending

- There is no build, clippy or enumeration receipt, so the runner refuses to execute.
- There is no GPU execution, adapter record, report, PNG or timing.
- There is no R4 appearance judgment. Older retained images were not used as R4 results.
- Cooked-temporal and native MetalFX paths are not covered (default WGSL path only).
- There is no phone, performance or memory claim.

**Screenshots:** none.
