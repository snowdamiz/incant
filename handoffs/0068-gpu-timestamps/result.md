# 0068 result — native GPU timestamp execution and unchanged appearance

## Status

**Executed under root's grant. Rendering is unchanged. An implementation defect
blocks timestamp acceptance and is routed to Astra for source review.**

I ran every bounded case in the frozen plan on one Apple M5 Pro (Metal). I then
released the GPU and the Cargo target. No source was changed or recompiled after the
defect was found.

- **Numeric verdict, lifecycle and bookkeeping: pass.** Actual GPU submissions
  confirm the following:
  - out-of-order submission still yields encoding sequences 1 and 2;
  - an unsubmitted command buffer is abandoned and releases its slot;
  - full-pool backpressure skips instrumentation while rendering continues;
  - deferred readback completes;
  - slot reuse works, and the drain order is exact;
  - every sample carries exactly the group labels its scene encodes, and absent
    groups stay absent (5 scene/fixture shapes, 8 samples).
- **Numeric verdict, timestamp values: fail (defect).** Every resolved timestamp is
  **0**: frame start and end, and every present group, in all 8 samples.
  - The brief's numeric test passes its own assertions (1/1, exit 0) because they
    only require finite, non-negative durations and the right labels.
  - An absolute GPU counter of 0 at every sample point is not quantization. No real
    GPU timestamp was observed, yet the API published these as valid zero-length
    measurements.
  - I preserved the result, did not rerun the test, and report the exact reproducer
    under "Defect for Astra". **Actual GPU timestamp values remain unestablished.**
- **Regression suites: 90 of 90 pass.**
  - The 89 existing ignored cases ran serially: 32 renderer library, 50 `model_gpu`,
    5 `gpu_host` game and 2 `gpu_host` UI.
  - The numeric test ran exactly once on its own, and the library suite skipped it.
  - The listings match the reviewed base exactly (33 + 50 + 5 + 2), so there is no
    integrated-source count change.
- **Appearance verdict: unchanged.** Explicitly enabling GPU timing changes no
  rendered pixel in the cases I tested:
  - 12 new frames were rendered, and all 4 retained timing-off/on pairs are
    byte-identical;
  - every frame matches its control, including a frame rendered with
    instrumentation skipped under backpressure and one rendered in a reused slot;
  - I inspected every retained frame myself and found them complete, with no
    artifacts.
- **`owner_drop`:** passes as the approved no-panic/no-hang lifecycle smoke check
  only. It does not establish callback-after-drop ordering.
- **Not approved here:** reference-phone performance, any other GPU backend, the
  complete profiler or any phase gate. The sample durations are all zero and are
  not interpreted.

## Brief, grant and director feedback

- **Brief.** SHA-256 `8e276281b2b52d0b33bcb3565ad3551855b5dbc6841010c075fe81758d8089a9`,
  committed by Astra in `71e4d63e`. There is no uncommitted root revision. I
  acknowledged and applied its priority update:
  - execution is authorized after review, using only the committed helpers, the
    frozen plan and the granted source;
  - all original failures are preserved, and I stop for source review on a defect.
    I compiled nothing new after the defect; the remaining frozen plan steps ran
    with the binaries frozen before execution;
  - `owner_drop` is approved as a smoke check only;
  - no personal, persistent or auto-memory edits. **This session made none.** The
    earlier preparation session, before this instruction existed, added two notes
    to Claude auto-memory. I have not touched them since.
- **Grant.** `/Users/andreyyurlov/.codex/incant-evidence/0068/review-ready.json`,
  SHA-256 `b07d3062da34d6bf880dd2ca01d4e61dbdb191569977cce755976742266fe1af`,
  granted 04:32:38Z.
  - It names packet `0068-gpu-timestamps`, the expected target, `source_commit`
    `71e4d63ed3515e82151aeb7237bef642c1b4bc02` and `approved_extra_cases: ["owner_drop"]`.
  - All three pinned prior receipts verify:

    | Pinned receipt | SHA-256 |
    |---|---|
    | 0067 `target-lease.json` | `bd68fcfe…` |
    | `cluster-grid-readback-002/gpu-released.json` | `5e88575b…` |
    | 0067 `gpu-released.json` | `07c185dd…` |

  - `lease.py` checks live-process fields only in the first release body (0067). I
    checked `cluster-grid-readback-002` by hand: `owned_processes: []` and
    `ledger_pids_alive: []`. The 0067 lease shows `released_at` and no processes
    on the target.
  - The grant copy is in `evidence/gpu/grant-copy.json`. I did not modify the grant.
- **Helper review.** `/Users/andreyyurlov/.codex/incant-reviews/0068-helper-source-final-20261013.json`
  matches the grant's `f6ab1b16…`: "accepted bounded execution helpers" at
  `f676b69e`. HEAD's tools and plan are byte-identical to `f676b69e`; only
  `brief.md` differs.

## Model and thinking effort

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** Max, the session's configured reasoning effort.
- **ACP verification.** Line 2 of root's
  `/Users/andreyyurlov/.codex/incant-evidence/claude-0068-execution-acp.log` reads
  `Claude session verified: Opus 5.5; thinking: max`. I read only that line.
- No subagent or auxiliary reviewer was used.

## Host, adapter and toolchain

| Item | Value |
|---|---|
| OS | macOS 27.0.1 (26A434), Darwin 27.0.0 arm64, `Mac17,9` |
| GPU / adapter | Apple M5 Pro, `IntegratedGpu`, backend `Metal` (wgpu 29.0.4) |
| Adapter timestamp features | `TIMESTAMP_QUERY` yes, `TIMESTAMP_QUERY_INSIDE_ENCODERS` yes, `TIMESTAMP_QUERY_INSIDE_PASSES` **no** (stage-boundary sampling only) |
| Device features | `Renderer::new`: no timestamp feature. `Renderer::new_with_gpu_timing`: both encoder-timestamp features |
| Timestamp period | `queue.get_timestamp_period()` = **1.0**, wgpu-hal's fixed non-Intel Metal value, not calibrated. wgpu-core timestamp normalization is inactive for period 1.0 |
| Rust | rustc 1.99.0 (b940084d7 2026-09-28), LLVM 23.1.1; cargo 1.99.0 (5f94df478 2026-08-27) |
| Python | 3.14.8, `-I -B` |
| Cargo target | `/Users/andreyyurlov/.codex/incant-core-presentation-target`, exclusive. Leased 04:33:54.51Z, released 04:42:20.68Z with no process on it. Left as warm cache |

## Source and frozen binaries

- **Isolated child checkout.** A `git archive` export of `71e4d63e` into
  `incant-evidence/0068/checkout`:
  - 14,423 files, every one blob-verified, with the tree
    `0950c0786767b090bfd141e24ce80b735691748b`;
  - manifest SHA-256 `f9980c2f…a1a8f6cf`;
  - every file was re-verified before each child.
- **Freeze.** All 10 files in the reviewed freeze (`d41ba28b…`) are identical, and
  `Cargo.lock` matches `c6831578…`.
  - There is no build-input diff from implementation `99778642` across `crates`,
    `Cargo.*`, `third_party`, `tools/cargo` and `rust-toolchain.toml`.
  - Source-level `#[ignore]` count: 83 renderer plus 7 GPU host = 90.
- **Builds.** All used `./tools/cargo … --locked --offline --profile ci -j2`, ran as
  bounded process groups, and never ran a built binary.

  | Build | Cargo args | Time | Result |
  |---|---|---|---|
  | `tests-render` | `test -p incant_render --lib --test model_gpu --no-run` | 38.3 s | 346 artifacts, 321 fresh |
  | `tests-host` | `test -p incant_render_gpu_host --test game --test ui --no-run` | 30.8 s | — |
  | `review` | `test -p incant_render --test gpu_timestamp_review --no-run` | 1.6 s | Linked the **identical fresh** `incant_render` library artifact as `tests-render` (`libincant_render-acc3d1929421bfc9.rlib`) |

  - The review helper compiled on its first build with one warning: an unneeded
    `mut` on `report` in `probe` (line 953). It has no behavioural effect.
- **Frozen binaries.** Copies live in `incant-evidence/0068/bin`. Each was
  hash-checked before and after every child, and none changed.

  | Binary | SHA-256 | Features |
  |---|---|---|
  | `incant_render-lib-tests` | `db939b04bcacf6606c9a285c18d4cd7a4c4c068c6b81fe6e8e72ef04ce5e736f` | `default`, `physics-deterministic` |
  | `model_gpu` | `49ebe070dc2d5125e54f777f8a84cfacec1dfd439f4201208d769b452b3c045e` | `default`, `physics-deterministic` |
  | `gpu_host-game` | `036a1ea84bf3767901fb56899f475aa94a0c94d79d4b13f3bf8a78c695e796d8` | host's `physics-deterministic` library build |
  | `gpu_host-ui` | `f5816d19348594b4da434ffcb432431f5be532049faec6fa9ce9e4fd9ed84387` | same |
  | `gpu_timestamp_review` | `349c8109c94dfcc2a6d2341c3afdf790b88ccde8a07b46bd5cadbe85faed9b2c` | `default`, `physics-deterministic` |

- **Tools.** Commit `71e4d63e`, plan SHA-256 `477082b0…`, helper SHA-256
  `611cb108…` (the reviewed `f676b69e` content).

## GPU interval, ledger and release

- **Ledger.** 18 children (`gpu-ledger.jsonl`, SHA-256 `1ca53607…`), serial, each in
  its own process group. Every group was empty afterwards, and no scratch files or
  wait/parse errors were left behind:
  - 3 builds;
  - 5 listings, at most 0.3 s each;
  - the numeric test (0.5 s);
  - 5 review cases, at most 1.2 s each;
  - 4 suites (1.6 s, 44.3 s, 0.6 s, 0.6 s).
- **Budgets used:**
  - appearance: **12 of 24** frames (all 640×360), with 7 PNGs retained;
  - numeric: **11 of 48** submissions (numeric test 3, `lit` 5, `sprites_text` 1,
    `viewport` 1, `owner_drop` 1);
  - no reruns and no plan amendments.
- **GPU latch.** `gpu-released.json` (SHA-256 `eba4f536…`) at 04:41:40.33Z, with
  no owned processes and no live ledger PIDs.
- **Lint.** The optional CPU-only lint ran afterwards, under the lease, and wrote to
  `post-release-ledger.jsonl`.
- **Lease release.** `target-lease.json` (SHA-256 `bb32a241…`) at 04:42:20.68Z.
- **Post-release refusals.** After release, `gpu.py review lit`, `gpu.py numeric`
  and `build.py tests` each refuse ("GPU and target already released by this
  packet") and are logged in `refusals.jsonl`.

## Numeric results

### The brief's numeric test (exactly once)

- **Run.** The frozen `incant_render-lib-tests` ran the test with the brief's exact
  libtest arguments, from `crates/incant_render` in the child checkout:

  ```
  timing::gpu_tests::bounded_timestamp_queries_follow_real_submission_and_abandonment --ignored --exact --nocapture --test-threads=1
  ```

  The binary was built by `tests-render`, which uses the brief's package selection
  and profile.
- **Result.** `test result: ok. 1 passed; 0 failed; … 102 filtered out; finished in
  0.41s`.
- **Adapter.** `AdapterInfo { name: "Apple M5 Pro", device_type: IntegratedGpu,
  backend: Metal, … }`.
- **Raw samples** (stderr, verbatim values):

  ```
  {"sequence":1,"timestamp_period_ns":1.0,"frame":{"start_ticks":0,"end_ticks":0,"nanoseconds":0.0},"passes":[{"start_ticks":0,"end_ticks":0,"nanoseconds":0.0},null,{"start_ticks":0,"end_ticks":0,"nanoseconds":0.0},{"start_ticks":0,"end_ticks":0,"nanoseconds":0.0},null,null,{"start_ticks":0,"end_ticks":0,"nanoseconds":0.0}]}
  {"sequence":2, … identical zeros …}
  ```

- **Checks against the test fixture.** The independent analysis compared these
  samples with the fixture's content-derived groups:
  - present: `InitializeScene`, `Shadows`, `Models`, `Display`;
  - absent: `AssignLights`, `Sprites`, `Text`.

  The labels match, the sequences are [1, 2], and conversion and ordering hold
  trivially. **Every tick is 0.**

### Review numeric checks (frozen helper, per-case `report.json`)

| Case | Helper checks | Groups present (= expected from content) | Samples |
|---|---|---|---|
| `probe` | 8/8 pass | n/a | Default device has no timestamp feature; stats disabled; `take` is none. The timing device has both encoder flags; stats start empty; `take` is none |
| `lit` | 14/14 pass | `InitializeScene`, `AssignLights`, `Shadows`, `Models`, `Display` | Seq 1, 2, 3, then none, then seq 5. All ticks 0 |
| `sprites_text` | 7/7 pass | `InitializeScene`, `Sprites`, `Text`, `Display` | Seq 1, then none. All ticks 0 |
| `viewport` | 7/7 pass | `InitializeScene`, `AssignLights`, `Shadows`, `Models`, `Display` | Seq 1, then none. All ticks 0 |
| `owner_drop` | 4/4 pass | n/a | One pending before submit; retained-device wait `QueueEmpty` after owner drop; fresh owner empty |

The `lit` stats sequence matches the pre-registered values exactly:

| Point | pending / ready / skipped |
|---|---|
| After frames 1 to 4 | 0/1/0, 0/2/0, 0/3/0, then **0/3/1** (frame 4 skipped instrumentation) |
| After the drain | 0/0/1 |
| After frame 5 | 0/1/1 |
| Final | 0/0/1 |

Abandoned and retired stayed 0 throughout.

**Independent re-checks** (`analyze.py`, CPU only) agree with the helper:

- all 7 PNGs decode to the helper's RGBA hashes;
- all 4 retained pairs differ in 0 pixels;
- re-derived labels match in all 8 samples, and conversion matches the period;
- there are no dependency-order violations, which is trivial with zero ticks;
- `observation_zero_tick_groups` lists every present group in every sample.

## Defect for Astra: every resolved GPU timestamp is zero

**Reproducer**, on the hardware above:

```
./tools/cargo test --locked --profile ci -j2 -p incant_render --lib \
  timing::gpu_tests::bounded_timestamp_queries_follow_real_submission_and_abandonment \
  -- --ignored --exact --nocapture --test-threads=1
```

The `GPU timestamp result:` lines show `start_ticks: 0, end_ticks: 0` for the frame
and every group. The packet helper's `lit`, `sprites_text` and `viewport` cases show
the same zeros across every group type it encodes:

- render passes: diagnostic initialization, shadow, model, sprite, text, display;
- the cluster-assignment compute pass;
- the trailing dummy-blit samples.

**What I read in the pinned sources (read-only; nothing tested with new code):**

- **Encoder writes.** `wgpu-core 29.0.4 command/query.rs:403–418, 241–258`: an
  encoder-level `write_timestamp` calls `reset_queries(index..index+1)` and then hal
  `write_timestamp`.
- **Deferral on this adapter.** Without blit/draw/dispatch-boundary sampling,
  `wgpu-hal 29.0.4 metal/command.rs:755–806` defers each encoder timestamp. It is
  then sampled in one of two ways:
  - at the start of the next render pass (start of vertex) or compute pass (start of
    encoder): `:961–978`, `:1679–1684`;
  - otherwise, at end of encoder in a dummy blit encoder, which the next
    `reset_queries`/`enter_blit` creates (`:157–250`).
- **Resolve.** The resolve is `resolveCounters` into the resolve buffer, inside the
  same command buffer (`:821–853`).
- **Normalization.** It returns early for period 1.0
  (`timestamp_normalization/mod.rs:107`), so it cannot be what zeroes the values.

**Hypotheses for source review (untested; each needs new code):**

1. The Metal counter samples are never written on this macOS 27 / M5 Pro
   configuration.
2. `resolveCounters` in the same command buffer runs before the samples are visible.
3. The resolve is a no-op, leaving wgpu's zero-initialized resolve/readback buffers.

Possible diagnostics, for Astra to decide:

- resolve in a later command buffer, or on the CPU through the counter sample
  buffer after completion;
- compare pass-level `timestamp_writes`, which needs only `TIMESTAMP_QUERY`;
- run wgpu's upstream timestamp test on this host.

**Contract and test gaps** (a source decision; I made no change):

- **Contract.** `recorder::decode` and `span` accept `0..0` everywhere, so an
  all-zero payload is published as a valid 0 ns frame. That is indistinguishable
  from the documented "legitimate quantization". A typed refusal for impossible
  samples, or a backend refusal, may be needed. This is the opposite of relaxing
  validity checks.
- **Test.** `bounded_timestamp_queries_follow_real_submission_and_abandonment`
  passes on all-zero values. It cannot detect unwritten queries, for example
  because it never asserts that `frame.end_ticks > frame.start_ticks` or that ticks
  are nonzero.

**Budget note.** 12 appearance frames and 37 numeric submissions of this packet's
budget were unused when I released the GPU. Any follow-up needs a new grant.

## Appearance verdicts (personal inspection of complete, unmasked frames)

| Frames (byte-identical within each set) | Content checked |
|---|---|
| `lit-off-1`, `lit-on-1`, `lit-on-4-skipped` (frame 4 rendered with instrumentation skipped under backpressure) | Shadow look-dev scene: cube, sphere, thin wall, sloped ramp with post, post row to the horizon. Consistent sun shadows with contact darkening. Warm (left) and cool (right) clustered point-light pools on the ground. Dark clear colour above the horizon. No missing geometry, seams, banding artifacts or black/NaN pixels |
| `sprites-text-off`, `sprites-text-on` | Pixel-perfect 2× grid. `ffi` ligature, right-to-left joined Arabic سلام, CJK 中文 and the intended `.notdef` box for the uncovered Hebrew letter; crisp 48 px "Ag". Three nearest-sampled badges with transparent corners and dark rims. The flipped third badge shows its marker mirrored and draws over the second |
| `viewport-off`, `viewport-on` | The lit scene inside the fractional offset rect `[52.5, 30.25, 512, 288]` with 12 px rounded corners over canvas `[64,80,96]`. Correct aspect, no boundary or corner artifacts |

Not visible in any frame, but confirmed by hash: the timing-on frames 2, 3 and 5, and
the second-device control in `lit`. All are byte-identical to `lit-off-1`.

These are observations of the unchanged renderer and fixtures, not new findings:

- the ground reads bright and the sky is a flat clear colour, because the
  environment lights the scene but is not drawn as a background;
- with timing enabled, the appearance stays the same.

## Screenshots

Unedited byte copies; each file stands for its byte-identical set:

| File | SHA-256 | Stands for |
|---|---|---|
| `handoffs/0068-gpu-timestamps/screenshots/lit-r1-lit-off-1.png` | `4f019b64…` | `lit-off-1`, `lit-on-1`, `lit-on-4-skipped` |
| `handoffs/0068-gpu-timestamps/screenshots/sprites_text-r1-sprites-text-off.png` | `bdadd584…` | `sprites-text-off`, `-on` |
| `handoffs/0068-gpu-timestamps/screenshots/viewport-r1-viewport-off.png` | `d6215b53…` | `viewport-off`, `-on` |

All 7 retained PNGs remain in `incant-evidence/0068/captures/<case>-r1/`.

## Helper lint (optional, CPU only, after the GPU latch)

`cargo clippy -p incant_render --test gpu_timestamp_review … -- -D warnings`
**fails (exit 101) on 4 style findings in the packet helper:**

- `unused_mut` at line 953;
- `clippy::chunks_exact_to_as_chunks` at lines 429 (twice) and 450.

None changes behaviour. I left them unfixed because the helper is the reviewed,
executed source. They would need fixing before any integration.

## Commands run (worktree root, `/opt/homebrew/bin/python3 -I -B`)

```
handoffs/0068-gpu-timestamps/tools/lease.py acquire
handoffs/0068-gpu-timestamps/tools/build.py checkout
handoffs/0068-gpu-timestamps/tools/build.py toolchain
handoffs/0068-gpu-timestamps/tools/build.py tests          # background; both cargo builds exit 0
handoffs/0068-gpu-timestamps/tools/build.py review
handoffs/0068-gpu-timestamps/tools/gpu.py list lib|model_gpu|game|ui|review
handoffs/0068-gpu-timestamps/tools/gpu.py numeric
handoffs/0068-gpu-timestamps/tools/gpu.py review probe|lit|sprites_text|viewport|owner_drop
handoffs/0068-gpu-timestamps/tools/gpu.py suite lib|model_gpu|game|ui
handoffs/0068-gpu-timestamps/tools/analyze.py check
handoffs/0068-gpu-timestamps/tools/gpu.py release
handoffs/0068-gpu-timestamps/tools/build.py lint           # exit 101, see above
handoffs/0068-gpu-timestamps/tools/lease.py release
handoffs/0068-gpu-timestamps/tools/{gpu.py review lit, gpu.py numeric, build.py tests}   # refused after release
handoffs/0068-gpu-timestamps/tools/analyze.py publish
```

Every child's exact argv, constructed environment overrides, hashes and outcome are
in `evidence/gpu/ledger.jsonl` and `evidence/gpu/post-release-ledger.jsonl`.

## Changed paths (this session)

- `handoffs/0068-gpu-timestamps/result.md`, replaced with this execution report.
  Astra's preparation follow-up is preserved verbatim below.
- `handoffs/0068-gpu-timestamps/evidence/**`, 56 published receipts plus
  `manifest.json`. Logs, JSON and JSONL were normalized only for trailing
  whitespace and final blank lines; the manifest records the raw and committed
  SHA-256 of each.
- `handoffs/0068-gpu-timestamps/screenshots/*.png`, 3 files.

No tool, plan, crate, test, CI or `main` change. Nothing was pushed. There is no PR.
Writes outside the worktree went only to the granted
`incant-evidence/0068` and the granted Cargo target.

## Limitations and unavailable checks

- **No timestamp values.** No real GPU timestamp value, and so no interval duration,
  was observed. See the defect section. This packet does not establish that the
  feature measures anything on Metal/Apple silicon.
- **Weak numeric test.** The numeric test's pass is not timestamp evidence, because
  its assertions accept all-zero values.
- **`owner_drop` scope.** It is a no-panic/no-hang smoke check only. `queue.submit`
  may deliver the callback before `drop(renderer)`, so callback-after-drop ordering
  is not established, and nothing exposes callback completion.
- **Untested failure paths.** I produced no real device mapping failure or device
  loss. CPU failed-map tests are not evidence of either.
- **Unsupported adapters.** The unsupported-capability refusal is not exercisable
  on this adapter, which supports encoder timestamps. The CPU test covers it.
- **Scope.** Only this Mac's Metal backend was tested. There is no Vulkan, DX12,
  browser, phone or performance evidence, and none is claimed.

## Open questions

1. Astra: which diagnostic seam should determine why resolved timestamps are zero,
   and should `decode` refuse impossible all-zero samples? A follow-up GPU interval
   needs a new grant.

## Astra correctness follow-up (2026-10-13, no grant or GPU)

The independent read-only helper review found that interruption of either child
wait killed the group but skipped reaping and terminal-ledger construction.
`build.py::run_group` and `gpu.py::run_child` now defer any original wait exception
until a `finally` kill/reap and a flushed, fsynced terminal ledger row. Timeout/exit
races are accepted as already-dead groups. An interrupted GPU attempt retains its
full reserved frame/submission budget and counts as an attempt; the original
exception is re-raised. A nonempty group refuses success. This is catchable Python
exception cleanup, not a guarantee against an uncatchable SIGKILL of the runner.

Seven CPU-only actual process controls pass (both runners' success, timeout,
SIGINT; plus interrupted GPU postprocessing). They execute `/usr/bin/true` or
`/bin/sleep`, inspect terminal rows and independently verify PID/process-group
absence. The GPU-runner controls also prove that charged attempts block a next
child exceeding the budget. Grant/source lookups alone are temporary test stubs;
no renderer, Cargo compiler, target lease or GPU work runs. Exact source hashes
and rows are in `preparation/lifecycle-checks.json`, with the lossless `lifecycle-checks.log.gz` beside it. All test processes were reaped. This follow-up was implemented by Astra.

`owner_drop` is suitable only as the documented bounded owner destruction and
retained-device completion smoke. The earlier phrase "before its mapping callback"
is not a proven temporal ordering: `queue.submit` may deliver callbacks before
`drop(renderer)`. The Rust comment now states this explicitly. A fresh timing
owner must start empty; no callback-after-drop, device loss or map-failure proof
is inferred. Actual execution and Claude appearance remain pending root grant.
