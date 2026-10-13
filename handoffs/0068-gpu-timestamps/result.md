# 0068 result — native GPU timestamp execution and unchanged appearance

## Status

**Prepared; execution pending root grant.**

- **Grant absent.** I checked the grant path once at session start. The directory
  `/Users/andreyyurlov/.codex/incant-evidence/0068/` did not exist, so
  `review-ready.json` is absent.
  - 0067 holds its own grant (`0067/review-ready.json`, granted 03:06:45Z), so 0067
    owns the GPU and the shared target.
  - I did not poll, wait or start another process to get around the queue.
- **Nothing executed.** I did not compile, acquire a target, run GPU work or inspect
  any new pixels. No cargo or rustc process ran, no GPU process started, and nothing
  under `incant-evidence/0068` was created.
- **Prepared for review:**
  - packet-only helpers;
  - a frozen, bounded case list (`plan.json`);
  - a preparation self-check, which passes.
- **Nothing claimed.** There is no GPU query result, regression-suite run, numeric
  verdict or appearance verdict yet. Root resumes this session after reviewing the
  helpers and creating the grant.

## Brief, grant and director feedback

- **Brief.** SHA-256 `ae7189cc6b0b158245fddd178b3390c91febd38d53ae4bb079ce991f90d70c00`.
  - Astra committed it in `9d4da6a9`, `dbf4c53f` and `d7309faa`. It is identical to the
    packet text in the session prompt, and there is no uncommitted root revision.
  - The only revision is the "Preparation stage while another packet owns the GPU"
    section (`d7309faa`). I applied it: preparation only, and this result is marked
    as above.
  - There is no separate director-feedback section to acknowledge.
- **Grant.** Absent, as described under Status.

## Model and thinking effort

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** Max, the session's configured reasoning effort.
- **ACP verification.** Line 3 of root's
  `/Users/andreyyurlov/.codex/incant-evidence/claude-0068-preparation-acp.log` reads
  `Claude session verified: Opus 5.5; thinking: max`. I read only that line.
- No subagent or auxiliary reviewer was used.

## Source contract, verified read-only

| Item | Result |
|---|---|
| Implementation | `99778642585a7b545a9448b9fca4a365ba622985`, base main186 `1c5658cf…` |
| Freeze file | `docs/spikes/evidence/gpu-timestamps/freeze.json`, SHA-256 `d41ba28b…a8f25` (matches the brief) |
| Frozen inputs | All 10 files in this worktree match `freeze.json`. HEAD `d7309faa` has no diff from `99778642` under `crates`, `Cargo.*`, `third_party`, `tools` or `rust-toolchain.toml` |
| Source review | `gpu-timestamps-source-20261013.json`, SHA-256 `2c0090c0…66f30a2`, `accepted_source`, no open findings |
| Archive | `artifacts.json.gz`, SHA-256 `ba0f5491…6229e` |
| Ignored GPU cases (source count) | 90 `#[ignore]` attributes: 33 `incant_render` library (including the new numeric test), 50 `model_gpu`, 5 `gpu_host` game, 2 `gpu_host` UI. Handoff 0066 executed 32/50/5/2 = 89 on the previous base |

I read the timing module (`timing/{mod,pool,recorder,gpu_tests,tests}.rs`), the graph
instrumentation (`graph.rs`) and the renderer entry points (`lib.rs`). The group
encoding conditions I rely on are:

| Group | Encoded when | Source |
|---|---|---|
| `InitializeScene`, `Display` | Every frame | `graph.rs` systems |
| `Models`, `Shadows` | The scene has model batches | `prepare_models` returns `None` for empty batches |
| `AssignLights` | Models are present, local (point/spot) lights > 0, and selection is `Clustered` (the default) | `lighting/mod.rs` `clustered.then(...)` |
| `Sprites` | Sprites are present | `sprites.prepare` returns `None` without them |
| `Text` | Text has glyph quads | `text.prepare` |

The helper derives expected labels from scene content (`SceneStats` plus the authored
local lights), not from the timing code.

### Pre-registered backend reading (pinned wgpu-hal 29.0.4, Metal)

I recorded this before any execution, so it cannot become a later excuse:

- **Feature flags.** On Metal, `TIMESTAMP_QUERY` and `TIMESTAMP_QUERY_INSIDE_ENCODERS`
  are both reported when the device supports stage-boundary counter sampling
  (`adapter.rs:1056`). Apple-silicon support is expected; the run will record the
  actual flags.
- **Deferred samples.** Without blit/draw/dispatch-boundary sampling, an
  encoder-level `write_timestamp` is deferred to the next pass. It lands on that
  pass's start-of-vertex (render) or start-of-encoder (compute) sample. Trailing
  samples go to a dummy blit encoder before the resolve (`command.rs:755–806,
  157–250, 518–530`). Consequences:
  - a group's end can coincide with the next group's start;
  - a group with no pass of its own can legitimately read zero ticks;
  - groups are not hardware barriers and must not be summed.
- **Period.** wgpu-hal uses a fixed period: `83.333` for Intel, otherwise `1.0`
  (`adapter.rs:90`). This is not a calibrated measurement, and the run will record
  `queue.get_timestamp_period()` as reported.
- **What decides acceptance.** Only the documented contract counts: labels match the
  encoded groups, absent groups are absent, spans are ordered, finite and
  non-negative, plus sequence, stats, backpressure and slot reuse. Dependency-order
  consistency, containment within the frame and zero-tick groups are recorded as
  observations only.
- **Reversed counters.** A reversed or wrapped counter would surface as a typed
  error. It would be reported to Astra with the exact reproducer. I would not relax
  the check or re-run to hide it.

## Proposed bounded case list (frozen in `plan.json`)

Budget totals:

- **Appearance frames:** 12 of 24 on first runs, and at most 24 even if every case
  used its one permitted rerun. Every review frame counts, including controls and
  frames whose PNG is not kept. Each frame is 640×360, within the 960×540 limit.
- **Numeric submissions:** 11 of 48 on first runs, and at most 19 with reruns.
  Timing-enabled review frames count here as well.

| Item | What runs | Frames | Numeric submissions | Bound |
|---|---|---|---|---|
| `numeric` | The brief's test, `timing::gpu_tests::bounded_timestamp_queries_follow_real_submission_and_abandonment`, with `--ignored --exact --nocapture --test-threads=1`, exactly once | 0 | 3 (2 more encoded, never submitted) | 120 s |
| `suite lib` | Library ignored cases, with `--skip` on the numeric test so it cannot run twice. The runner checks that `running N` equals the listing minus one and that the name is absent from output | existing | existing | 300 s |
| `suite model_gpu`, `game`, `ui` | All ignored cases, serial, `--nocapture` | existing | existing | 300 s each |
| `review probe` | Timing on and off: adapter/backend, adapter and device timestamp flags, period, initial stats and `take`. The default device must request no timestamp feature | 0 | 0 | 120 s |
| `review lit` | Look-dev shadow scene (sun with shadows, environment fill, two clustered point lights), 640×360. See "The lit case" below | 8 | 5 | 120 s |
| `review sprites_text` | Pixel-perfect camera (100 px/unit, 320×180 → 640×360), three nearest sprites (one flipped, overlapping), world text (`MIXED` Latin/Arabic/CJK/.notdef plus `Ag`). One frame off, one on, exact bytes | 2 | 1 | 120 s |
| `review viewport` | The lit scene in viewport rect `[52.5, 30.25, 512, 288]`, radii 12, canvas `[64,80,96]`. One frame off, one on, exact bytes | 2 | 1 | 120 s |
| `review owner_drop` | **Gated; needs Astra's review first.** Submit one measured 64×64 frame (the numeric-test fixture, not readable), drop the owning `Renderer`, wait on a retained device clone, then check that a fresh timing renderer starts empty | 0 | 1 | 120 s |

### The lit case

- **Timing off.** Two frames on one device (determinism control) and one on a second
  device (cross-device control).
- **Timing on:**
  1. three measured frames leave three unread results in the fixed slots;
  2. frame 4 renders with instrumentation skipped;
  3. draining yields sequences 1, 2, 3 and then none;
  4. frame 5 reuses a slot (sequence 5).
- **Checks.** All eight frames must be byte-identical to the first. Stats after each
  step must equal pre-registered values, and every sample's labels must equal the
  content-derived set.

### Further rules

- **Kept frames.** Four matched pairs are retained, within the brief's 3–6:
  - `lit-off-1` / `lit-on-1`;
  - `lit-off-1` / `lit-on-4-skipped`;
  - `sprites-text-off` / `-on`;
  - `viewport-off` / `-on`.
  Every rendered frame is hashed whole and compared byte for byte in-process. The
  kept PNGs are also decoded and compared independently in Python.
- **Reruns.** A review case may run a second time only in one of two cases:
  - after an infrastructure failure, meaning a timeout or a crash before
    `report.json` exists;
  - after a committed `plan.json` amendment that names the case and the reason, for
    example a helper assumption defect visible in the first run's raw data. This
    follows 0067's lesson to keep a reserve.

  Every run stays in the ledger, a recorded failure is never hidden, and the 24/48
  caps still bind. The numeric test and the suites never rerun.
- **Builds.** Builds (CPU only, 2700 s bound each) use the plan's three cargo
  invocations, all with `--locked --offline --profile ci -j2`:
  - `tests-render`: `-p incant_render --lib --test model_gpu --no-run`. This keeps
    the brief's package selection for the numeric test.
  - `tests-host`: `-p incant_render_gpu_host --test game --test ui --no-run`.
  - `review`: `-p incant_render --test gpu_timestamp_review --no-run`. This runs
    only after the regression binaries are frozen. It must reuse the identical fresh
    `incant_render` library artifact.

## Execution order once granted (all `python3 -I -B`, from the worktree root)

```
handoffs/0068-gpu-timestamps/tools/lease.py acquire
handoffs/0068-gpu-timestamps/tools/build.py checkout      # git archive export of grant source_commit, blob-verified
handoffs/0068-gpu-timestamps/tools/build.py toolchain
handoffs/0068-gpu-timestamps/tools/build.py tests
handoffs/0068-gpu-timestamps/tools/build.py review
handoffs/0068-gpu-timestamps/tools/gpu.py list lib|model_gpu|game|ui|review
handoffs/0068-gpu-timestamps/tools/gpu.py numeric
handoffs/0068-gpu-timestamps/tools/gpu.py suite lib|model_gpu|game|ui
handoffs/0068-gpu-timestamps/tools/gpu.py review probe|lit|sprites_text|viewport   # owner_drop only if approved
handoffs/0068-gpu-timestamps/tools/analyze.py check       # CPU only; then personal inspection of every kept frame
handoffs/0068-gpu-timestamps/tools/gpu.py release
handoffs/0068-gpu-timestamps/tools/build.py lint          # optional: clippy -D warnings on the helper, CPU only, still leased
handoffs/0068-gpu-timestamps/tools/lease.py release
handoffs/0068-gpu-timestamps/tools/analyze.py publish
```

The numeric test runs as the frozen library test binary. That binary is a byte copy
of what `tests-render` built: the same package selection, profile and lock as the
brief's command. It runs with the brief's exact libtest arguments, from
`crates/incant_render` in the child checkout, which is the cwd cargo would use. No
test reads `CARGO_*` at runtime; I checked with grep.

## Helpers (all new, packet-only, standard library or test-only Rust)

| Path | Purpose |
|---|---|
| `tools/common.py` | Constants and guards. `grant()` refuses before touching the filesystem while the grant is absent. `gate()` adds the release latch and the lease. `tools_identity()` refuses uncommitted tool or plan edits. `verify_checkout()` re-checks every checkout file against its git blob before each child. Also holds the constructed child environment |
| `tools/lease.py` | Create-only target lease. Verifies every `prior_*_path`/`prior_*_sha256` pin in the grant and the prior release's empty live-process fields, and refuses if any cargo/rustc or target-run binary is active. Release leaves the target as a warm cache |
| `tools/build.py` | `checkout`, `toolchain`, `tests`, `review`, as described above, plus an optional `lint`. Lint runs after the GPU latch, under the lease, and writes to a separate post-release ledger so the latched ledger hash stays valid. The tool parses Cargo JSON artifacts, records features and hashes, and freezes executables into `0068/bin`. It never runs them |
| `tools/gpu.py` | Serial runner under an flock. Enforces budgets and run limits from `plan.json`, uses one process group per child with kill-and-reap on timeout, writes a ledger and refusal log, and writes the release latch |
| `tools/analyze.py` | CPU-only checks independent of the helper: its own PNG decoder, pair diffs, content-derived labels, sequence/order/tick-to-ns checks, and parsing of the numeric test's samples. Its publish step makes create-only copies and keeps one unedited PNG per distinct byte content |
| `tools/selfcheck.py` | Preparation check (below) |
| `tools/review/gpu_timestamp_review.rs` | The Rust review helper. It is copied into the child checkout as `crates/incant_render/tests/gpu_timestamp_review.rs` only by `build.py review`. It reuses `tests/materials` and `tests/support` and the look-dev geometry copied from `tests/shadows/lookdev.rs`, where those builders are private. All output goes to a new directory named by `INCANT_0068_REVIEW_OUT`, using create-only files |
| `plan.json` | Frozen cases, budgets, bounds and order |

No existing helper constructs a timing-enabled renderer:

- `examples/native_frames.rs`, `examples/frame_benchmark.rs` and
  `gpu_host/examples/ui_review.rs` all call `Renderer::headless()`/`new`.
- Pointing them at the timing constructor would change production example code.

So the packet adds one narrow integration-test file, not a capture framework.

**Receipt environment.**

- **Constructed environment.** Children get a constructed environment: `HOME` passes
  through with its value unrecorded, plus documented nonsecret overrides recorded
  verbatim: `PATH`, `TMPDIR` (a fresh packet scratch directory), `RUST_BACKTRACE=1`,
  and `CARGO_TARGET_DIR`, `CARGO_TERM_COLOR=never` and `RUSTUP_AUTO_INSTALL=0` for
  builds.
- **Review output variable.** Review children also get `INCANT_0068_REVIEW_OUT`.
- **Never serialized:** the inherited environment, ACP or session transport values,
  credentials and account metadata.
- **No optional evidence.** Regression tests cannot see any `INCANT_*_EVIDENCE`
  variable, so they write no optional evidence.
- **Pattern.** This follows the 0066/0064 pattern, made stricter. It does not reuse
  the 0065 logger.

## Requests to root and Astra (before or with the grant)

1. **Grant fields.** The tools require:
   - `packet: "0068-gpu-timestamps"`;
   - `target: "/Users/andreyyurlov/.codex/incant-core-presentation-target"`;
   - a 40-hex `source_commit`;
   - prior-receipt pins as `prior_<name>_path` plus `prior_<name>_sha256` pairs,
     including one whose name contains `release` (0067's grant format).

   Any `owned_processes`-style list in the pinned release must be empty.
2. **Child checkout approval.** My proposal is a `git archive` export of
   `source_commit` into `incant-evidence/0068/checkout`, verified file by file against
   the git blobs. The tree has 14,410 regular files and no symlinks or submodules. No
   build script uses git. The shared `.git` is not modified. If root wants a different
   isolated checkout, please say so in the grant; adapting `build.py` would be a small
   change committed before execution.
3. **`owner_drop`, Astra review.** The brief requires review of any
   callback-after-owner-drop helper. It is the `owner_drop` function in
   `tools/review/gpu_timestamp_review.rs`. To approve it, list `"owner_drop"` in the
   grant's `approved_extra_cases`; otherwise it is refused and reported as not run. It
   observes only the absence of a panic or hang and a fresh owner starting empty. It
   does not manufacture device loss or a mapping failure.
4. **Integrated source.** If the grant names a source other than `99778642`,
   `build.py checkout` records:
   - the build-input diff;
   - the comparison against the reviewed freeze;
   - per-file ignore counts.

   I would explain any count change from 90 before running suites.

## Commands run this session and results

| Command | Result |
|---|---|
| Read-only inspection: `git log/show/ls-tree/diff`, `grep`, `sed`, `shasum` on this worktree, the pinned wgpu 29.0.4 registry sources, and 0064/0066/0067 receipts and tools in other worktrees | No writes outside this worktree |
| `ls /Users/andreyyurlov/.codex/incant-evidence/0068/`, once | `No such file or directory`: grant absent |
| `rustfmt --edition 2024 --emit stdout < tools/review/gpu_timestamp_review.rs`, formatting only through stdin | Exit 0, so the file parses as Rust. I adopted rustfmt's layout, and a second pass is identical. No compilation |
| `/opt/homebrew/bin/python3 -I -B handoffs/0068-gpu-timestamps/tools/selfcheck.py --write handoffs/0068-gpu-timestamps/preparation/selfcheck.json` | `all_pass: true` |

What `selfcheck.json` confirms (Python 3.14.8):

- all tools parse;
- the plan totals add up and fit 24 frames, 48 submissions and 960×540;
- PNG names are unique and the four kept pairs name real PNGs;
- the numeric test is the library suite's only skip, and the reviewed base totals 90;
- the helper's five tests, frame and submission limits, PNG names, size and output
  variable match `plan.json`;
- all 15 guarded commands exit 1 with "review-ready.json is absent";
- `incant-evidence/0068` is still absent and `git status` is unchanged.

## Changed paths

- `handoffs/0068-gpu-timestamps/plan.json`
- `handoffs/0068-gpu-timestamps/tools/{common,lease,build,gpu,analyze,selfcheck}.py`
- `handoffs/0068-gpu-timestamps/tools/review/gpu_timestamp_review.rs`
- `handoffs/0068-gpu-timestamps/preparation/selfcheck.json`
- `handoffs/0068-gpu-timestamps/result.md`

I made no change to crates, production code, tests in `crates/`, CI, `brief.md` or
`main`. Nothing was pushed. There is no PR.

## Screenshots

None. No frame was rendered, and no pixel was inspected.

## Limitations and unavailable checks

- **Helper not compiled.** The Rust helper has not been compiled; compiling is
  forbidden before the grant. rustfmt confirms only that it parses. It mirrors API
  uses already present in the repository's tests. Any compile fix found by
  `build.py review` would be recorded as a reviewed deviation before any GPU child
  runs.
- **Only refusal paths tested.** The tools' grant, lease, build, GPU and publish paths
  are unexercised until the grant exists.
- **Unsupported adapters.** The unsupported-capability refusal cannot be exercised
  on an adapter expected to support timestamps. The CPU test
  `absent_capability_fails_explicitly_without_changing_defaults` covers it. Execution
  would record an actual refusal as a refusal, never as a pass.
- **Mapping failure.** No real device mapping failure will be produced. CPU
  failed-map tests are not evidence of it.
- **Out of scope.** This packet cannot approve reference-phone performance, other
  GPU backends (Vulkan, DX12, browser), the complete profiler or a phase gate. The
  sample durations will not be read as performance.

## Open questions

1. Does root approve the `git archive` child checkout, or name another one?
2. Will Astra review `owner_drop` and approve it via `approved_extra_cases`?
3. Is running the frozen copy of the `tests-render` library test binary, with the
   brief's exact libtest arguments, acceptable for the numeric command? The
   alternative is the literal cargo command after the freeze, with
   `Running`-path and before/after hash checks.
