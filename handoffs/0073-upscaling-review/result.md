# 0073 result — output upscaling numeric execution and visual-review preparation

## Status

- **StageA (granted once): executed and passed.** Both exact tests ran once on
  Apple M5 Pro (Metal) and exited 0. The GPU was released with no owned processes.
  - All 28 printed scalar rows match independently derived expectations; worst
    absolute error 1.46e-4.
  - The release, ledger and debit chain re-verifies.
  - Details: [`numerical-result.md`](numerical-result.md).
- **StageB: prepared only, not run.** The fixture, frozen plan and guarded tools
  are committed for a future root grant after source review.
  - No build, compile, GPU child, render or capture ran for this stage.
  - Details: [`preparation-result.md`](preparation-result.md).
- **Not claimed:** appearance, image quality, performance, a reference phone,
  MetalFX, the cooked-native path, or any phase-gate approval. The numeric pass
  is not used as appearance evidence.

## Model, effort, brief

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max. Root's
  `~/.codex/incant-evidence/claude-0073-upscaling-acp.log` header reads
  `Claude session verified: Opus 5.5; thinking: max`; I read only that header.
- **No auxiliary agents.** No subagent or reviewer was used.
- **Brief:** `handoffs/0073-upscaling-review/brief.md`, SHA-256 `13d7b351…73a1`,
  committed by root in `3caadfee`.
  - It has no priority revision or added director feedback, and it was unchanged
    at the final commit.
  - I applied its constraints: the single StageA launch; preparation only for
    StageB; no push, PR, memory edit or credential access; Built-by: claude.
- **Files left to root:** `numeric.py` and the brief are root's and are
  unmodified.

## Commands and results

| Command | Result |
|---|---|
| `python3 -B handoffs/0073-upscaling-review/tools/numeric.py` (once, console captured create-only in the grant directory) | Exit 0. Child 1: 1.155 s, exit 0. Child 2: 0.391 s, exit 0. Both report `1 passed; 0 failed`. Released: 2 children, `owned_processes: []`, `ledger_pids_alive: []` |
| `python3 -I -B handoffs/0073-upscaling-review/tools/numeric_receipt.py` | 11/11 chain checks true; 28/28 rows within 1.5e-4. 11 raw outputs retained as deterministic gzip; the evidence directory was unchanged |
| `python3 -I -B …/visual/tools/selfcheck.py --write …/preparation/selfcheck.json` | `all_pass: true`. All 12 guarded commands refuse without a grant and touch nothing |
| `python3 -I -B …/visual/tools/test_lifecycle.py --receipt …/preparation/lifecycle.json` | 17/17 (real children; debit, timeout, INT/TERM/HUP, mask, refusals, release) |
| `python3 -I -B …/visual/tools/test_analyze.py --receipt …/preparation/analyze-selftest.json` | Pass, 6/6 |
| `rustfmt --edition 2024` via stdin on the fixture | Parses; formatted; `--check` clean. No Cargo target or compilation |

**Unavailable by design.** These were not run, because the brief forbids them in
this stage:
- compiling the fixture;
- clippy;
- any GPU appearance run;
- regression suites.

## Screenshots

None. StageA reads back scalar rows only, and StageB has not run. The 13 planned
PNGs (960×540) are listed in `visual/plan.json`; after a grant they will be
published to `handoffs/0073-upscaling-review/screenshots/`.

## Changed paths

- `handoffs/0073-upscaling-review/numerical-result.md`
- `handoffs/0073-upscaling-review/tools/numeric_receipt.py`
- `handoffs/0073-upscaling-review/evidence/numeric/` (`receipt.json` and 11 `<sha256>.gz`)
- `handoffs/0073-upscaling-review/visual/` (plan, fixture, 8 tools, 3 preparation receipts)
- `handoffs/0073-upscaling-review/preparation-result.md`
- `handoffs/0073-upscaling-review/result.md`

**Commits:**
- `98b4d266`: StageA record.
- `290c1e35`: StageB visual tree `3e328c47…`.
- The commit adding this file and `preparation-result.md`.

Every commit carries `Built-by: claude`.

**Outside the repository**, I added only the create-only console capture
`runner-console.{stdout,stderr}` beside root's StageA outputs in
`~/.codex/incant-evidence/0073-upscaling-numeric/`. Nothing was pushed.

## Rationale

**StageB visual direction (mine).** One 960×540 physical output is compared
across FULL, DRS 8/16 and UP 8/16, using identical crop rectangles.
- **Why this stage.** It concentrates the cases where output-resolution
  reconstruction differs by design:
  - edges and sub-texel detail that the shader keeps current-only;
  - accumulated interiors;
  - texture minification at internal resolution;
  - current-only overlays;
  - physical UI.
- **Jitter evidence.** The full 8-phase cycle is shown directly through phase
  strips, an activity map and per-phase wire presence.
- **Motion, scale changes and cuts.** These run as short sequences with exact
  FULL and DRS controls.
- **Budget.** 89 of 96 frames leaves a reserve, and 13 of 16 PNGs.
- **What decides a case.** Only deterministic contract facts decide pass or fail;
  image statistics are observations.

**Tooling.** The tools reuse 0068's reviewed helpers rather than building a new
framework. They are tightened where the brief asks:
- the debit is written and fsynced under the packet lock and the shared GPU lock
  that cover the launch;
- interrupts are held pending until the attempt is recorded;
- every child starts with an empty signal mask;
- a cooked-feature build is refused;
- a failure ends the stage.

**StageA observation.** In mode 7, two composite channels are stored one
half-float step below round-to-nearest, which is consistent with
round-toward-zero conversion. This is within the test's 1e-3 tolerance and is
not a defect claim.

## Open questions for root

1. **Target and pins.** Which `~/.codex/*-target` should the grant name, and
   which prior release(s) should it pin? StageA's `gpu-released.json` is
   `fe268a84…6678`.
2. **Compile first?** The fixture is uncompiled. Should the first grant allow
   only `lease`, `checkout`, `toolchain` and `review` (CPU), with GPU steps
   afterwards? As written, a compile error stops the stage before any GPU frame
   and needs a new tree.
3. **UI counting.** Should UI submissions keep counting against the 96 frames?
   This is conservative; there are 10 of them.
4. **Regression suites.** Should the existing ignored lib/`model_gpu` suites run
   with the same frozen inputs, budgeted separately?

## Limits

- **StageA:** one local Metal adapter and scalar readback. Its 22 actually
  submitted frames are derived from source and were not instrumented.
- **StageB:** an uncompiled fixture and no visual evidence. Crop placement is
  projected and unconfirmed until frames exist.
- **Not covered:** phones, performance, MetalFX, cooked-native artifacts and
  other backends. The 640 MiB + 16 KiB logical ceiling is not a phone budget.
