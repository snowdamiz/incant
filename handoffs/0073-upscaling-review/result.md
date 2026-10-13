# 0073 result — output upscaling numeric execution and visual-review preparation

## Revision 1 (current): fail-closed analyzer and corrected-source readiness

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
