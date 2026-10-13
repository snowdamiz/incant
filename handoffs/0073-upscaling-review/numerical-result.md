# 0073 StageA — output upscaling numerical execution

**Status: both exact tests passed once each; the GPU was released.** This is
scalar readback on one Metal adapter. It says nothing about appearance,
performance, phones or any phase gate.

## Model, brief and grant

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max.
- **ACP proof:** root's transport log
  `~/.codex/incant-evidence/claude-0073-upscaling-acp.log` reads
  `Claude session verified: Opus 5.5; thinking: max`. I read only its header.
- **Brief:** `handoffs/0073-upscaling-review/brief.md`, SHA-256 `13d7b351…73a1`, as
  committed by root in `3caadfee`. There was no uncommitted revision.
- **Grant:** `~/.codex/incant-evidence/0073-upscaling-numeric/review-ready.json`,
  SHA-256 `61065efe…6b05`. It pins:
  - runner `405864682d7b…81a0`, which equals `tools/numeric.py`;
  - predecessor release `0072-temporal-r3/gpu-released.json`, `0c63e503…7e2`:
    2 children, no live processes;
  - lifecycle helper `cluster-grid-readback-002/run-gpu.py`, `0f890ddb…166a`;
  - executable copy `afa3d035…4298`, mode `0700`.
- **Checks I made before launch** (read-only):
  - All of the above hashes matched.
  - No process held `~/.codex/incant-evidence/native-gpu.lock`.
  - No `incant_render`, cargo or rustc process was running.
  - HEAD was `3caadfee` with a clean tree.
  - Build inputs (`crates`, `Cargo.*`, `third_party`, `tools/cargo`,
    `rust-toolchain.toml`) were unchanged since `0452c5f5`.
- **Source review:** I read `docs/spikes/temporal-upscaling.md`, the root review
  `temporal-upscaling-verification.md` and `evidence/temporal-upscaling-review.json`,
  and both test sources before launch.

## Command and run

```
python3 -B handoffs/0073-upscaling-review/tools/numeric.py \
  > ~/.codex/incant-evidence/0073-upscaling-numeric/runner-console.stdout \
  2> ~/.codex/incant-evidence/0073-upscaling-numeric/runner-console.stderr
```

| Item | Value |
|---|---|
| Interpreter | `/opt/homebrew/bin/python3` 3.14.8 |
| Runner exit | 0; stderr empty |
| Wall clock | 06:40:00–06:40:01Z |
| Child 1 | pid 59620, 1.155 s, exit 0, no timeout |
| Child 2 | pid 59622, 0.391 s, exit 0, no timeout |
| Child environment | `PATH=/usr/bin:/bin:/usr/sbin:/sbin`, `TMPDIR=/tmp`, `RUST_BACKTRACE=1`, `HOME` (value not recorded) |
| Lock | shared `native-gpu.lock`, exclusive and non-blocking |
| Debits | written and fsynced before each child: 14 + 18 = 32 |
| Release | `gpu-released.json` `fe268a84…6678`: 2 children, `owned_processes: []`, `ledger_pids_alive: []` |

**Test 1:** `upscaling::system::device_tests::actual_output_reconstruction_rejects_bad_history_and_never_retains_overlay`

- `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 149 filtered out; finished in 0.85s`

**Test 2:** `models::retained::upscaling_tests::upscaling_color_motion_geometry_and_dynamic_source_extent_commit_together`

- `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 149 filtered out; finished in 0.37s`

After the run, both PIDs and their process groups were gone, and the lock was
free.

## Adapter

`AdapterInfo { name: "Apple M5 Pro", vendor: 0, device: 0, device_type:
IntegratedGpu, driver: "", backend: Metal, subgroup 4–64, … }`. The host is
`Mac17,9`, macOS 27.0.1 (26A434), Darwin 27.0.0 arm64. This identifies the
device; it is not a benchmark.

## What actually executed

**Test 1: 14 GPU submissions.** Each is an 8×8 fixture run:
- a depth clear, the real resolve and composite pipelines, and two copies;
- source rect `[0.5,0.5,3,3]` in a 4×4 source, output rect `[1,1,6,6]`;
- history weight 0.5.

It printed all 28 rows (14 modes × output history and composite), as RGBA16Float
values read at pixel (3,3).

**My comparison.** I compared every row with expectations I derived from the
fixture's semantics, in `tools/numeric_receipt.py`.
- **Result:** all 28 match; worst absolute error 1.46e-4.
- **Exact rows:** 25 of 28.
- **Half-float rounding, the other three:**
  - Mode 5 (R and G): 0.2 and 0.1 stored as the nearest f16.
  - Mode 7 composite (R and G): `0.28735352` and `0.23742676`. These sit one f16
    step below round-to-nearest of the exact 0.2875 and 0.2375, which is
    consistent with round-toward-zero storage.
  - The test's tolerance is 1e-3; this is an observation only.

| Mode | Fixture change | History / composite |
|---|---|---|
| 0 | compatible perspective history | accepted: 0.375 |
| 1 | orthographic depth 50.05 | accepted: 0.375 |
| 2 | prior depth 10 | rejected: current 0.5 |
| 3 | motion leaves the viewport | rejected: 0.5 |
| 4 | alpha-0 radiance 100 | zero |
| 5 | half coverage | current-only premultiplied `[0.2,0.1,0,0.5]` |
| 6 | prior 20 with full reaction | 0.5 |
| 7 | overlay `[0.1,0.05,0,0.5]` | history 0.375; composite `over + 0.5·history` |
| 8 | negative current R | cleaned, current-only `[0,0.5,0.5,1]` |
| 9 | NaN current | zero |
| 10 | negative history | rejected: 0.5 |
| 11 | NaN motion | rejected: 0.5 |
| 12 | no previous frame | 0.5 |
| 13 | HDR `[2,0.5,100]` both | preserved |

In every mode, the pixel outside the output rect read as zero bytes.

**Test 2: retained 65×33 lifecycle and DRS.** By its source, it makes 15
`submit` calls:
- 8 accepted GPU submissions: the first frame, the retry, the reset, the
  replacement and DRS 8/4/12/16;
- 7 refusals before any write or queue submission: the zero-width target, the
  `R32Float` format, the zero transform and four zero-width DRS probes.

The test prints no rows; the pass is its own assertions:
- the owner, epoch and sequence;
- jitter phases 0–4;
- the internal sizes `ceil(65·n/16) × ceil(33·n/16)`;
- history-slot alternation;
- a fresh owner on replacement.

**Total:** 22 actual submitted frames against the 32 debited. Both counts are
derived from source; neither test instruments its frame count.

## Retained evidence

`handoffs/0073-upscaling-review/evidence/numeric/` holds:
- `receipt.json`;
- 11 deterministic gzip copies named by raw SHA-256. They cover every runner
  output plus the grant: console, both child stdout/stderr, ledger, debits,
  results, release, summary, start record and grant. Both child stderr files are
  empty.

The receipt re-verified the chain, and all 11 checks are true:
- the release ledger and debit hashes;
- the child stdout/stderr hashes;
- the ledger equals the results;
- the exits;
- the binary hash;
- no live PIDs or groups.

Command: `python3 -I -B handoffs/0073-upscaling-review/tools/numeric_receipt.py`.
It wrote nothing under the evidence directory; a before/after listing was
identical.

## Limits

- One Metal adapter, scalar readback only, and no appearance.
- No performance, reference-phone, MetalFX or cooked-native evidence.
- `numeric.py` and the brief are root's files and are unchanged.
- No retry, no production edit, no Cargo build and no screen capture.
