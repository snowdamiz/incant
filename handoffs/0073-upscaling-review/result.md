# 0073 result — output upscaling numeric execution, visual-review preparation and visual execution

## Execution (current): corrected portable visual review

**Status: EXECUTED, and the GPU is released. Hard results PASS; my quality
verdict is FAIL.** Details are in [`visual-result.md`](visual-result.md).
- **Contract:** all 20 contract checks pass, and the fail-closed `analyze.py
  check` passes with no problems.
- **Quality:** the 8/16 upscaler converges to a stable but soft image.
  - Accumulation removes texture detail that the current internal frame still
    has; converged chart contrast is below plain DRS's.
  - Edges and sub-pixel geometry stay phase-unstable.
- **What works:** current-only overlays and physical UI are stable; history
  survives internal-only scale changes; the cut resets exactly; the fractional
  viewport is clean; there is no motion ghosting.
- Production is unchanged; the findings go to coordinated repair. This is not a
  phase completion, performance result or phone result.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-visual-execute-acp.log` header reads `Claude session
verified: Opus 5.5; thinking: max`. No subagents.

**Inputs (all hashes verified before the first GPU step):**
- brief `1fc2219d…` (root's `32d6d6bb`);
- visual tree `7e6d4b08…`;
- `review-ready.json` `ebf34321…`;
- `build/review.json` `b52c6918…`;
- frozen helper `786adfbb…`;
- root build receipt `6109ca8b…`;
- prior 0070 release `aa8fd844…`;
- engine source `535003d7…`, with freeze `bd8295b0…` exact over 2,448 build
  inputs.

This is the default portable WGSL path only; it is not cooked and not native
MetalFX.

**Commands and results.** All ran once, serially, in the foreground. The build
was root's supervised build: no Cargo, build, source edit, helper edit or target
access in this session.

| Command (`python3 -I -B …/visual/tools/`) | Result |
|---|---|
| `gpu.py list` | Exit 0 |
| `gpu.py review probe` | Exit 0, 8/8 checks; Apple M5 Pro, Metal |
| `gpu.py review stationary` | Exit 0, 36 frames, 5/5 checks |
| `gpu.py review motion` | Exit 0, 24 frames, 3/3 checks |
| `gpu.py review transitions` | Exit 0, 29 frames, 4/4 checks |
| `gpu.py release` | 89/96 frames, 13/16 PNGs; no live or owned process, no stop, no refusal |
| `analyze.py check` | Exit 0, verdict pass, complete |
| `analyze.py publish` | 37 receipts and 13 PNGs bound to `summary-2026-10-13T082346…` |

**Screenshots:** 13 PNGs at 960×540 in
`handoffs/0073-upscaling-review/screenshots/`:
- `stationary-{full,drs50,up50,phases,detail,activity}`;
- `motion-{full,up50,detail}`;
- `transitions-{full,up50,strip,cut}`.

**Changed paths:**
- `handoffs/0073-upscaling-review/visual-result.md`
- `handoffs/0073-upscaling-review/result.md`
- `handoffs/0073-upscaling-review/screenshots/` (13 PNGs)
- `handoffs/0073-upscaling-review/evidence/visual/` (manifest plus 37 files)

Outside the repository I added only the create-only `console/` captures in the
evidence directory.

**Not run:**
- retries;
- cooked-temporal or native MetalFX routes;
- other backends;
- regression suites;
- phones;
- performance.

**Open questions for root (coordinated repair):**
1. Should the portable resolve move to output-resolution sample accumulation,
   with an edge history policy and a texture LOD bias? Should the repaired build
   then be reviewed with this unchanged fixture, for a like-for-like comparison?
2. Should a later grant include a longer 16/16 segment, so that the review shows
   where 16/16 settles? Three frames were too short.
3. The fixture's physical UI label clips its own text. Should a later fixture
   revision fix that? It does not affect any contract check.

## Revision 1 (preserved): fail-closed analyzer and corrected-source readiness

**Status: PREPARED, not run.** The root review blocker is fixed, and the stage is
ready for root's supervised CPU build grant. Details are in
[`revision-result.md`](revision-result.md).
- No Cargo build, target, compile, GPU child, render or capture ran in this
  revision.
- No appearance, performance, phone or gate claim is made.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-analyzer-repair-acp.log` header reads `Claude session
verified: Opus 5.5; thinking: max`.

**Inputs:** brief `cd7c45b7…` (root priority revision `15ddda49`) and root review
receipt `d307b665…`; both hashes were verified.

**Applied:**
- **`analyze.py` fails closed.** It validates exact complete plan cases, the
  plan's contract checks, the current ledger, debit, report, PNG and log
  bindings, and every independent comparison. It always retains a diagnostic
  summary and exits nonzero on any problem. `publish` accepts only the latest
  passing summary whose bindings still match.
- **Source pinning comes from root grants.**
  - `build-ready.json` names a commit object and root's freeze by SHA-256.
    Every build input of the exported commit must match that freeze exactly.
  - `review-ready.json` binds the build record.
  - `cooked-*` and `native-metalfx` builds are refused.
- **Corrected source read-only at `9a65b6c8`.** The fixture's contract checks
  are independent of the changed math, so its source is unchanged. Three prose
  expectations are superseded: overlays (now unjittered), motion disocclusion
  and the viewport border.

**Evidence (CPU only, receipts in `visual/preparation/r1-*.json`):**
- the 17 unchanged lifecycle controls pass;
- 17 analyzer controls pass: one positive, the rest negatives (missing
  case/report/PNG, bad hash, replaced build record, false
  contract/tile/statistic/UI/chrome, failed child, stop latch, zero-verdict
  evidence, stale or superseded summary, CLI exit codes);
- 10 source-pinning controls pass;
- selfcheck reports `all_pass`.

**Frozen identity:** commit `9bcd1e0e`, visual tree `09271a62…`; fixture
`f8ec6311…` (unchanged).

**Screenshots:** none; nothing has been rendered.

**Root's next actions:**
1. The final committed corrected source and its freeze, generated from that
   commit. The current generation-2 freeze for `9a65b6c8` includes 11
   uncommitted rows and an uncommitted doc, so it would be refused.
2. `build-ready.json` for the supervised CPU build.
3. After the build, `review-ready.json`.

**Limits:**
- The fixture has not been compiled against the corrected source.
- This covers the portable path only.
- StageA stays historical evidence for the `0452c5f5` binary only.

## Revision 0 (preserved; only its headings are nested one level)

### Status

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

### Model, effort, brief

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

### Commands and results

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

### Screenshots

None. StageA reads back scalar rows only, and StageB has not run. The 13 planned
PNGs (960×540) are listed in `visual/plan.json`; after a grant they will be
published to `handoffs/0073-upscaling-review/screenshots/`.

### Changed paths

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

### Rationale

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

### Open questions for root

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

### Limits

- **StageA:** one local Metal adapter and scalar readback. Its 22 actually
  submitted frames are derived from source and were not instrumented.
- **StageB:** an uncompiled fixture and no visual evidence. Crop placement is
  projected and unconfirmed until frames exist.
- **Not covered:** phones, performance, MetalFX, cooked-native artifacts and
  other backends. The 640 MiB + 16 KiB logical ceiling is not a phone budget.
