# 0074 result — bounded native MetalFX ownership execution

## Preparation session — root priority revision (2026-10-13)

**Status: the five-case native visual review is prepared and CPU-checked, but not
executed.** This session ran no Cargo and used no target, GPU or screenshot. I
changed nothing outside this packet.

- Nothing can build or run until root grants an explicit combined source (see
  "Grant root needs to supply" below).
- The numeric record further down is unchanged. I did not run `numeric.py` again,
  and its six evidence files still hash exactly as recorded there.

### Model, effort and brief

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max, as
  configured for this session. No subagents were used.
- **ACP proof:** line 2 of root's ACP log
  `~/.codex/incant-evidence/claude-0074-visual-prep-acp.log` reads "Claude session
  verified: Opus 5.5; thinking: max". Its SHA-256 was `18fe7ca0…` when I read it; the
  log may still grow.
- **Brief:** SHA-256 `e8c598ce5d2e589c55b8c7540c7ce747e26cb63b2de1aa6dde1df2dd841d45da`.
  Root committed its priority revision in `89915700`, and the brief did not change
  during this session.
- **Root receipt:** `metalfx-0074-numeric-20261013.json` verified, SHA-256
  `d5869e4b…`, status `accepted-scoped-native-execution`.

### Root feedback, acknowledged and applied

- **Consumed grant:** not executed again.
- **Scope:** preparation only. I reused 0072's fixture and accepted helpers and added
  no execution framework, retry loop or broad suite. 0072's regression suites are
  removed, not carried over.
- **Budgets:**
  - 108 world frames + 11 UI = 119 submissions (limit 128);
  - 18 PNGs at 960×540 (limit 20);
  - one serial child per case;
  - failed attempts are charged and never retried.
- **Source:** not `4bc810fe` or `a337c86d`.
  - The tools refuse an absent grant.
  - They refuse those two commits and the source-003 freeze by name.
  - They require an explicit commit, its tree, a root-reviewed per-file freeze and
    one allowlisted feature set.
  - The combined head at preparation, `0c890514` ("integration checkpoint only"), is
    neither treated as granted nor assumed to contain the 0072 fixes.
- **Arms:** portable and native arms come from the same source. Every native frame
  must report `Selection::MetalFx`, with `RequireMetalFx` wherever content scale 1 is
  certain. Fractional and device-range fallbacks are explicit per-step expectations.
- **Current-only boundary:** reported as its own observation, never as vendor output.
  It is the set of pixels where the native frame equals the same-phase current-only
  control.
- **Jitter:** I accept root's correction.
  - My session-1 concern rested on the first sentence of the header alone.
  - Apple's WWDC22 session 10103 example pairs sample (0.625, 0.78) with API jitter
    (−0.125, −0.28). The API jitter is therefore the pixel centre minus the sample
    position, which is exactly the engine's image shift.
  - The header's second sentence ("…offset this scaler samples to return to the
    frame's reference frame") agrees.
  - I propose no sign change. The static registration observation stays as the
    numerical check, and I will judge the eventual pixels myself.
  - I relied on root's citation and did not fetch the video page.
- **Diagnostics:** I added none.
  - The native scale range is inferred from selection: a `Portable(Scale)` result
    would reveal it.
  - Native resets are inferred from selection changes, `has_previous`, epoch and
    content size (`inferred_resets`).
  - No extra observation is strictly necessary for these cases.

### What was prepared (all under `handoffs/0074-metalfx-review/visual/`)

| Path | Role |
|---|---|
| `plan.json` | Frozen case plan, budgets, source policy and base-fixture provenance |
| `review/metalfx_visual_review.rs` | Fixture extension; only `build.py review`, under a grant, copies it into `crates/incant_render/tests/` |
| `tools/{common,build,gpu,lease,analyze}.py` | 0072's accepted helpers, adapted |
| `tools/{selfcheck,test_lifecycle}.py` | 0072's CPU checks, extended |
| `preparation/{selfcheck,lifecycle}.json` | CPU-only receipts |

### Case plan

Cases run in this order, one child each; the first failure latches `stop.json`, so
the cases that rest on the most assumptions (transitions, viewport) run last.

| # | Case | Arms (frames) | Submissions | PNGs |
|---|---|---|---|---|
| 1 | stationary | off 1, current-only 8, portable 8, native (Require) 16 | 33 | 4 |
| 2 | motion | native 8, portable 8, current-only 8, off at steps 6–8; native pan 6, off 1 | 34 | 5 |
| 3 | excluded | native 8, portable 2, off 1, each followed by a UI submission | 11 + 11 UI | 3 |
| 4 | transitions | one Prefer owner: 16/16 ×4, cut ×2, 12/16 ×3, 8/16 ×3, 10/16 ×2, 16/16 ×2; off at 16/16 and 8/16 | 18 | 4 |
| 5 | viewport | Prefer owner in an integral viewport: 16/16 ×6, 8/16 ×3; off 1; fractional viewport (Prefer) ×2 | 12 | 2 |
| | **Total** | | **119** (reserve 9) | **18** (reserve 2) |

**Hard checks** (pass/fail):

- **All cases:** native selection on every native frame; no MetalFX owner on the
  controls; planned frame counts.
- **stationary, motion, excluded:** uninterrupted runs.
- **excluded:** the UI rectangle stays within 2 codes of off on every frame, and the
  scene contains BLEND, sprites and text.
- **transitions:** each step has the planned selection (`Native`, `NativeOrScale`
  or `FractionalViewport`); one owner throughout; the cut restarts at phase 0 and
  advances the epoch; the planned internal sizes.
- **viewport:** planned selections; pixels outside the viewport (beyond a 1 px band)
  byte-identical to off; the 8/16 internal size.

**Observations** (measured, never pass/fail):

- **Registration:** a Lucas-Kanade shift against the off frame and its slope on the
  applied image shift. This is the jitter check.
- **Current-only boundary band** (stationary and motion).
- **Disocclusion trail masks** (motion): pixels that changed from off step 7 to 8,
  and those that changed only since step 6.
- **Inferred native resets** (transitions).
- **Unreconstructed 8/16 control** (transitions).
- **Consecutive change and activity maps.**

### Helper adaptation (line diff against the 0072 blobs)

- **`common.py`** (−29/+86):
  - new packet, stage and paths;
  - `source_problem()`, the grant policy described above;
  - `samples()`, a jitter-table parser;
  - a fix for 0072's latent crash when the grant lacks a `target`.
- **`build.py`** (−67/+73):
  - checkout verifies the granted tree, every file in root's freeze, and the
    fixture's jitter copy against the granted source;
  - the review build uses exactly the granted features and rejects a library with a
    partial cooked feature set;
  - the regression-suite build is removed.
- **`gpu.py`** (−57/+32): runs the review binary only (`list review`, the five cases
  in plan order, `release`); suites removed.
- **`lease.py`** (−9/+11) and **`analyze.py`** (−3/+4): identity, rules and budgets.
- **`selfcheck.py`** (−31/+282) and **`test_lifecycle.py`** (−7/+80): the extended
  checks listed below.
- **Fixture:**
  - 63 reused items are byte-identical to 0072's blob `b363f64c…`.
  - Adapted: the `Report::open` message, `open_renderer` (now `enable_metalfx`),
    `Output` (viewport), and `Meta`/`Shot`/`shoot`/`same_owner` (upscaling metadata
    plus selection).
  - New: the arms, cases and observation helpers.
  - Removed: the probe, skinned and timing cases, `Rig` and the timing helpers.

### CPU-only checks (no Cargo, target or GPU)

- **`selfcheck.py --write …/preparation/selfcheck.json`:** exit 0, `all_pass: true`.
  - Plan consistency and budgets.
  - An independent model of the frame, upscaling and MetalFX plans classifies all 25
    steps, including the 13 native-owner steps, as planned. Internal sizes:
    960×540, 720×405, 480×270 and 600×338.
  - The fixture agrees with the plan: test names and order, limits, PNGs per case,
    preferences, configs, viewports and internal sizes.
  - 63 reused items are identical, and every tool descends from its 0072 blob.
  - All 15 grant-policy cases behave as expected.
  - rustfmt 1.10.0 (edition 2024, stdin to stdout, calibrated on the 0072 blob)
    leaves the fixture byte-identical.
  - With the grant absent, all 12 guarded commands refuse; no evidence directory is
    created, `git status` is unchanged and the global lock is untouched.
- **`test_lifecycle.py --receipt …/preparation/lifecycle.json`:** 19/19 pass (14
  adapted lifecycle tests and 5 source/guard tests), 2.6 s, no leftover children.
- **Mutation check** (in memory, not committed): each of these was detected:
  - a changed reused item;
  - a wrong footprint expectation;
  - `RequireMetalFx` on an uncertain scale;
  - a renamed PNG;
  - a budget change;
  - a changed jitter table;
  - a superseded source.
- **Fixed during preparation:** with stdin input, `rustfmt --check` exits 0 even when
  it prints a diff, so the format check compares bytes instead.

### Grant root needs to supply

The grant goes in `~/.codex/incant-evidence/0074-metalfx-visual/review-ready.json`
and must contain:

- `packet: "0074-metalfx-review"` and `stage: "metalfx-visual"`;
- the full `source_commit` and its `source_tree`;
- `source_freeze: {path, sha256}`, with an absolute path to a freeze that lists files
  under `sources` and, optionally, `unchanged_baselines`;
- `features`: either `["native-metalfx"]` or `["cooked-temporal", "native-metalfx"]`;
- `visual_tree`: the tree of the committed `visual/` directory;
- an allowed `target` matching `~/.codex/*-target`;
- at least one `prior_<name>_path` / `prior_<name>_sha256` release pin, for example
  this packet's numeric `gpu-released.json` (`55a72bba…`) or a later GPU release.

### Limitations

- **Not compiled:** clippy was not run either; root supervises compilation.
  - I checked the fixture by inspection against `4bc810fe`'s API (ancestry
    `e73afe43` → `4bc810fe` → `0c890514`), by rustfmt parsing and by the selfcheck.
  - The combined source may still change APIs; the build step will show it.
- **Possible fallbacks:** the 8/16 and 12/16 native frames may legitimately fall back
  with `Scale` on this device. Such frames are recorded and excluded from the native
  observations.
- **No pixels yet:** every appearance judgment, including the jitter registration,
  remains open.

### Changed paths, transients and screenshots

- **Added:** the 11 files under `handoffs/0074-metalfx-review/visual/`.
- **Edited:** this `result.md`, with this section only. `brief.md`,
  `tools/numeric.py` and the numeric record below are unchanged.
- **Transients:**
  - `/tmp/0074-selfcheck-dry.json`, a dry-run output, deleted;
  - Python temporary directories that the lifecycle tests created and removed;
  - a `.fmt` scratch file inside the packet, moved over the fixture.
- **Evidence directory:** nothing was written to `~/.codex/incant-evidence`.
- **Screenshots:** none (preparation only).
- **Not done:** no memory writes, personal notes, accounts, credentials, pushes or
  PRs, and no phase approval requested or implied.

---

## Numeric stage record (session 1, unchanged)

## Status

**Executed once under root's grant and passed. MetalFX was actually admitted and
selected on an Apple M5 Pro (Metal). The GPU was released.**

This run measures ownership, fallback, reset and cache identity only. It makes no
pixel, appearance, vendor-quality or performance claim, and it approves no gate.

- **Result:** the exact ignored test passed (1 passed, 0 failed) with exit 0. Stderr
  was empty, and libtest reported 1.29 s.
- **Adapter:** `Apple M5 Pro`, `IntegratedGpu`, `backend: Metal`, with
  `admission: Some(Ok(()))`. Portable fallback was not counted as success: the test
  asserts native selection.
- **Assertions:** all passed, in this order:
  1. The first submit selected `MetalFx`.
  2. A deliberately refused `R32Float` target left the selection, sequence (1), live
     instances and accepted color unchanged.
  3. The next native submit advanced the sequence to 2.
  4. A fractional viewport selected `Portable(Some(FractionalViewport))` and kept the
     identical cached native texture.
  5. Returning to native reused that texture.
  6. `invalidate_motion_history` gave `!has_previous` and a later epoch.
  7. A fresh `RequireMetalFx` owner selected `MetalFx`, with a motion owner distinct
     from the first owner's.
- **Printed counts:** `native submissions=5; portable submissions=1; rejected
  submissions=1` is literal text printed after the assertions pass. It shows that
  execution reached the end; it is not a runtime counter. The two sequence numbers
  are runtime values:
  - `old sequence=5`: 4 native and 1 portable accepted submissions on the first
    owner. The refused attempt did not advance it.
  - `fresh sequence=1`: the replacement owner's single submission.
- **Release:**
  - no signals were sent, and no other process joined the group;
  - the leader was reaped, the group was absent, and there were no survivors;
  - `gpu-released.json` records `success: true` and `owned_processes: []`;
  - afterwards no process held `native-gpu.lock`, and none was running the frozen
    binary.
- **After the run:** preparation only (proposal below). No further GPU work, build or
  image.

## Model, effort and routing

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), with thinking/reasoning
  effort **max**, as configured for this session. I can observe these settings only
  from inside the session. The ACP-side verification record belongs to root.
- **Delegation:** no subagent, other model or auxiliary reviewer was used.

## Brief, grant and pins

- **Brief:** SHA-256 `ef1044a9d9669a9e2a85ef8449f521696c0a1c10c573b3364eaee11f7c0f6e80`,
  committed in routing commit `3777806f`. It was unchanged at start, before the run
  and before my commit, and it contains no priority revision or new director
  feedback. No files from an interrupted attempt existed: the packet held only
  `brief.md` and `tools/`, and the evidence directory held only the grant.
- **Grant:** `~/.codex/incant-evidence/0074-metalfx-numeric/review-ready.json`,
  SHA-256 `3a7d6907c026fcfb78157de92227acea8a8469e7d5801d6eec43a650aaecc2d7`. Limits:
  one child, 60 s, 6 frames at most 64×64, 16 command buffers, `target: null`.
- **Pins:** I checked every pin myself, read-only. The runner's `--preflight` mode
  then re-checked all of them except the two review receipts:

| Pin | SHA-256 | Result |
|---|---|---|
| Runner `tools/numeric.py` | `02563b7377b8b005bfef241a82da80f06beabbc055caf7078e15722d7bf5fd63` | match |
| Freeze `native-metalfx-source-003/freeze.json` | `b68c62d5e58b89a07aebfd17f144902917cee5890c4d09fbf06f51a64146a77b` | match; all 23 sources and 10 baselines are byte-identical in this worktree |
| Manifest `docs/spikes/evidence/native-metalfx.json` | `1f0773a76f7276502f0a7ed9cd514af4f3f55a6959d27e003748065737c8f6b6` | match |
| Binary `native-all-incant_render-867026fbadfe8587` (32,071,488 B) | `53ecd261e03fb21655519969123806d49d3489645ce4bf1e504f9d54ed23e671` | match; executable; exact test name occurs once |
| Lifecycle helper (`0070-ambient-occlusion/tools/lifecycle.py`) | `574db13f25dc27939b99becb7fdb5272b6b5734bcda694487d1e42b127e764a0` | match |
| Predecessor `0072-temporal-visual/gpu-released.json` | `d2342da2d2cd88a74f5a94a17c2cb16eedb893f14709ad8ab7fabdfc9eda66af` | match; ledger `c66b02e6…` has 21 rows for 21 children, none alive |
| Independent source review | `105e456508d7ec75361f886734c43535e92100ba34567f5992635bd68c61f85b` | accepted, scoped, no findings |
| Root source/CPU final | `3de769385be7566e897d94ba1bc2d8779a22ebad4af9371685bc9a6c3b7a89ce` | accepted |

- **Binary features:** the manifest records the `native-all` check as `cargo test -p
  incant_render -p incant_render_gpu_host --features
  incant_render_gpu_host/native-metalfx`, with no `cooked-*` feature. The pinned
  binary is that check's generation-003 build. So `motion::admission()` should not
  make the test return early before reaching MetalFX, and the printed adapter and
  admission line confirms that it did not.
- **Interpreter:** `python3` resolves to Homebrew 3.14.8. This matters because
  `lifecycle.run` uses `datetime.UTC` (Python 3.11+) only after `attempt.json` exists.
  `/usr/bin/python3` is 3.9, so it would have consumed the one-shot grant and then
  failed.
- **Commits:** HEAD `3777806f` is source `4bc810fe0daf238f219801de76b06f19e4daf826`
  plus root's two packet files only.

## Run

Both commands ran from the worktree root, in the foreground:

1. `python3 -I -B handoffs/0074-metalfx-review/tools/numeric.py --preflight` → exit 0,
   output `Source, manifest, binary, exact test and predecessor release verified; no
   GPU execution.`
   - This is the runner's own no-GPU mode. It only reads and hashes: it takes no lock,
     writes nothing (the evidence directory was unchanged) and does not consume the
     grant.
   - I ran it once, so a pin failure would surface without spending the one-shot.
     Root may judge whether it counts against the one-command grant.
2. `python3 -I -B handoffs/0074-metalfx-review/tools/numeric.py` → exit 0, output
   `Exact MetalFX ownership regression passed; GPU released.`
   - The child ran the frozen binary with `<exact test> --ignored --exact --nocapture
     --test-threads=1` in the worktree.
   - Its environment contained only `PATH`, `HOME`, `TMPDIR` and `RUST_BACKTRACE`.

Complete child stdout (606 bytes; the leading and trailing blank lines are part of
the file):

```text

running 1 test
test models::retained::metalfx_tests::actual_native_batch_and_fallback_preserve_retained_submission_ownership ... MetalFX adapter: AdapterInfo { name: "Apple M5 Pro", vendor: 0, device: 0, device_type: IntegratedGpu, device_pci_bus_id: "", driver: "", driver_info: "", backend: Metal, subgroup_min_size: 4, subgroup_max_size: 64, transient_saves_memory: true }; admission: Some(Ok(()))
native submissions=5; portable submissions=1; rejected submissions=1; old sequence=5, fresh sequence=1
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 161 filtered out; finished in 1.29s

```

Child stderr: empty (0 bytes).

**Lifecycle outcome:**

- Child PID/PGID 76821, uid 501, kernel start `1791876259.537525`. Comm `python3.14`
  is the spawn-gate trampoline before exec; exec keeps the same identity.
- Gate opened; exit 0; no signal; not timed out.
- `signals_sent: []`, `members_seen: []`, `survivors: []`, `reaped: true`,
  `group_absent: true`.
- Timeline (UTC): attempt 07:24:19.537; child ran 07:24:19.537–07:24:21.141; release
  07:24:21.154.
- System: `macOS-27.0.1-arm64-arm-64bit-Mach-O`.

**Evidence** (root's directory; only root's runner wrote here):

| File in `~/.codex/incant-evidence/0074-metalfx-numeric/` | SHA-256 |
|---|---|
| `attempt.json` (6 frames, 16 command buffers, 64×64) | `370ae7589a70b69439823bbf814410f67e0b36a7be62b7cc688b4bbd212f69bf` |
| `ledger.jsonl` (spawn + terminal) | `02417bbb294e4941da6720a45451d5cfeef1b269b1f9dd0a50be61a581d11a7e` |
| `native.stdout` | `a604ef3d7ae8e517e5ffcdf6fc2165cf76a1539284cf229ab7c0b0ea3910383e` |
| `native.stderr` (empty) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `gpu-released.json` | `55a72bba5f7f930ab0e86b9906db10d840e4ea7ce29ed894316161173ea13bfd` |
| `tmp/` | empty directory created by the runner |

## What the run establishes, and what it does not

**Established, on this device and OS only:**

- MetalFX is admitted and actually selected.
- The three-buffer native submit completes, five times across two owners.
- After the refused format attempt, no state is published.
- Fractional-viewport fallback reports its exact reason and keeps the one bounded
  native cache.
- Resume reuses that cache, an explicit reset advances the epoch, and replacement
  creates a new owner.

**Not established:**

- **Content scale was 1:1.** No dynamic resolution was set, so source and output were
  both 64×64. MetalFX ran as native-resolution anti-aliasing, not upscaling. The run
  shows only that this device admits scale 1.0. The lower end of its admitted range
  was not observed.
- **Paths not exercised on the GPU:**
  - integral offset viewports, which use the native path;
  - three-in-flight `Busy` saturation (the test waits after every submit);
  - native content-size (DRS) resets;
  - `RequireMetalFx` refusal.
- **Output not inspected:** no pixel was read back, so MetalFX output values,
  coverage normalization and overlay composition remain unverified on the GPU.
- **Not measured:** Apple's internal memory, timing and performance.
- **Other targets:** no iOS or reference-phone run.

## Remaining appearance and acceptance gates

All of these remain open; none is claimed here:

- Native image quality against current-only and portable controls (see the proposal).
- Pixel confirmation of the jitter, motion and depth conventions, above all the jitter
  sign (proposal item 3).
- Native output at content scale < 1, native DRS resets, and integral offset
  viewports on the GPU.
- Root's pending 0072 shimmer/overlay correctness fixes. They are not in source
  `4bc810fe`, and nothing here treats this source as fixed.
- Native cooked combinations (refused by design; root owns the cooker work).
- Reference-phone/AWS Device Farm runs and performance. These need the plan's
  shipping-profile device measurements.
- Phase gates, which only humans can approve. None is requested or implied.

## Proposal: a later bounded native visual review (preparation only)

The review should run only on root's reviewed shared fixtures, after the 0072 fixes.
It should add arms to that harness, not a new framework. The 0072 harness
(`temporal_visual_review.rs`: `shoot`, `edges`, `consecutive`, `split`, `activity`,
`rect_delta`) already has the stage, mover, excluded-overlay scene and 960×540
readback this needs.

**Source facts it depends on** (source `4bc810fe`):

1. **The edge policy is duplicated; overlay compositing is shared.**
   - `metalfx/composite.wgsl` (the native normalize pass) re-implements the portable
     resolve's current-only rule for clear, fractional and mixed-depth 2×2
     footprints. It shares only the helpers in `upscaling/common.wgsl`.
   - A 0072 fix to `upscaling/resolve.wgsl` therefore does **not** reach the native
     path unless it is ported.
   - A fix to `upscaling/composite.wgsl` (overlays) does, because native frames run
     that same composite pass.
   - Pin both shader hashes in the review.
2. **Native anti-aliasing is discarded at depth edges.** Wherever a 2×2 footprint
   touches clear coverage or incompatible depth, the native frame shows the current
   frame's jittered bilinear color, not MetalFX output. Silhouettes should therefore
   shimmer the way portable edges do. That is engine policy, not vendor quality.
3. **The jitter sign is the main unverified convention.**
   - The engine's jitter is an image shift: `temporal/tests.rs` asserts that projected
     points move by `+SAMPLES[phase]` pixels, right/down.
   - `Owner::prepare` passes that value, in content pixels, straight to
     `jitterOffsetX/Y`.
   - The pinned header describes the property as "the subpixel sampling coordinate you
     use to generate the color texture input". Under an image-shift convention, that
     coordinate is the negated value.
   - The two conventions can differ in sign, and I cannot settle it from source. A
     wrong sign would misregister history by up to 0.75 px (2 × 0.375). It would show
     as persistent swimming or blur on stationary thin edges.
4. **Motion and depth match the header's stated conventions.**
   - `prepare.wgsl` writes current→previous displacement in content pixels with both
     jitters removed. That matches the sign of the header's example: an object moving
     down-right by 10 px has motion `(-10,-10)`.
   - Depth is forward [0,1), with `depthReversed = false`.
   - Exposure is a 1×1 texture cleared to 1.0, with `preExposure` 1 and auto-exposure
     off.
   - Input color is linear premultiplied HDR, up to 65504.
5. **Native history resets more often than portable.** It resets on every
   content-size change and every selection change (`History::reset`,
   `Owner::prepare`). Portable history survives internal DRS changes. The first
   native frame after a DRS step will probably look softer than portable's. Record
   that as expected behavior, not a defect.
6. **Only the depth tolerances reach MetalFX.** Of `TemporalConfig`, the native path
   uses only `relative_depth` and `absolute_depth`, in the normalize edge test.
   `history_weight` and the `reactive_*` fields do not affect MetalFX; its temporal
   weights are opaque.

**Arms.** Every case uses one renderer with `Renderer::enable_metalfx()` at startup;
that call also enables portable upscaling and accumulation. Every arm starts from a
fresh owner, so jitter phases line up by frame index. Confirm this with equal
sequence numbers.

- **O (off):** the plain retained scene, no temporal (0072's `plain`).
- **C (current-only):** a portable upscaling owner with `history_weight: 0`, which
  `TemporalConfig::validate` accepts. All other `REVIEW` fields are unchanged. This
  arm isolates jitter and the current-only edge rule from accumulation.
- **P (portable):** `enable_temporal_upscaling(REVIEW)`, the reviewed portable path.
- **N (native):** `enable_metalfx(&r, Preference::RequireMetalFx, REVIEW)`. Assert
  `metalfx_selection() == Some(Selection::MetalFx)` after every native frame, so
  fallback can never count as native. Use `PreferMetalFx` only in steps planned to
  fall back, and assert the exact `Portable(reason)` there.
- **Build:** `incant_render/native-metalfx` with no `cooked-*` feature.

**Cases.** Frame counts are proposals; root sets the hard budget.

| Case | Content | Arms × frames | Targets in source | PNGs |
|---|---|---|---|---|
| 1 Stationary | 0072 stage: thin wall/grille, MASK card, one HDR highlight > 1 against dark | O1, C8, P8, N16 = 33 | jitter sign/registration, convergence, MASK sparkle, HDR fireflies | O, P f8, N f16, N activity f9–16 |
| 2 Motion and disocclusion | 0072 mover crossing the thin wall and MASK card; camera pan | object N8, P8, C8, O at steps 6–8; pan N6, O1 = 34 | motion sign/scale, jitter removal, reactive mask, forward depth | N/P f8, O8, N pan f6, O pan |
| 3 Transitions | one N owner (Prefer): 16/16 ×4 → cut (`invalidate_motion_history`) ×2 → 12/16 ×3 → 8/16 ×3 → 10/16 ×2 → 16/16 ×2; O at view B 16/16 and at 8/16 | 16 + 2 = 18 | reset on cut and content size, `FractionalViewport` at 10/16, reset on resume, cache reuse | first frame after cut, after 12/16, after 8/16, and on resume |
| 4 Offset viewport | integral `[96,54,768,432]` at 16/16 and 8/16 (`[48,27,384,216]`); fractional `[96.5,54.5,767,431]` | N6 at 16/16, N3 at 8/16, O1, Prefer ×2 = 12 | zero-offset packing, `lo`/`hi` bounds, offset readback in normalize | N f6 (full frame), O |
| 5 Excluded overlays | 0072 excluded scene: BLEND model, sprites, world text, screen UI | N8, P2, O1 = 11 frames, each with a UI submission | overlays never enter MetalFX; shared composite; UI stays outside history | N f8, O, N activity |

Total: 108 frames, 11 UI submissions and 18 PNGs, plus a reserve.

Footprints at 960×540:

- 12/16 → 720×405 and 8/16 → 480×270 are integral and keep the aspect ratio.
- 10/16 → 600×337.5 is fractional, so it is the planned fallback.
- The offset viewport becomes fractional at 12/16 (y offset 40.5).
- If the device's minimum scale excludes 0.5, the 8/16 steps report `Scale`. Record
  that as a device fact, and exclude those frames from native observations.
- Validate every planned footprint on the CPU before the first child (lesson from
  0067).

**Contract checks** (pass/fail, as in 0072):

- Native selection on every N frame, and exactly the planned fallbacks.
- `continuous` and `same_owner` runs, with an epoch advance at the cut.
- Outside the viewport (beyond a 0.75 px band), N equals O byte for byte.
- The UI rectangle matches O on every frame.
- Frame, UI and PNG counts exactly match the plan.

**Observations** (measured, never pass/fail):

1. **Registration.** Find the sub-pixel profile centroid of the stationary thin
   horizontal and vertical edges in N f16 and compare it with O. Also measure
   consecutive change. This settles source fact 3.
2. **Band split.** Per frame, pixels where N equals C exactly mark the engine's
   current-only band. From source, both arms produce identical bilinear color there.
   Report consecutive change separately for that band and for the rest, which is
   vendor output. Flat interiors can match by coincidence, so intersect with the 0072
   edge mask.
3. **Disocclusion.** The band is the pixels the mover covered in O6 or O7 but not in
   O8. Compare N8, P8 and C8 with O8 there: maximum and mean delta, and any stale
   ghost of the mover.
4. **Moving thin content.** Compare the trail behind the mover's edges across N, P
   and C, and look for breakup along the MASK card's edges.
5. **Resets.** Compare each first frame after a reset with O. N against P at the DRS
   steps should show the expected softness difference from source fact 5.
6. **HDR.** Look for fireflies or ringing at the bright highlight in N versus P.
7. **Overlays.** Compare N and P inside overlay regions at the same phase, and look
   for halos at overlay alpha edges.

## Changed paths

- `handoffs/0074-metalfx-review/result.md`: this file, and the only file I committed.
- `brief.md` and `tools/numeric.py` are root-owned and untouched.
- Outside the worktree I wrote only a transient scratch file,
  `/tmp/0074-diffcheck.txt` (the `git diff --cached --check` output), and deleted it
  immediately. Root's runner wrote the evidence files listed above.

## Screenshots

None. The grant excludes screen capture and images, and no PNG was produced or
inspected.

## Not run or unavailable

- **No Cargo, build, fmt, clippy or other test.** None was granted, and no source
  changed.
- **No pixel readback** and no appearance, performance, iOS or reference-phone run.
- **No retry.** The single granted execution passed. No source edits.
- **No background jobs.** None was started or left running.

## Open questions and requested data bindings

1. **Jitter sign** (proposal fact 3). Root and Astra can settle it through the
   visual review's registration check, or from source before the review.
2. **Scale range not observable.** The device's admitted MetalFX scale range is
   private (`metalfx::native::range`). A read-only diagnostic next to
   `metalfx_admission` would let the evidence record it; for now only the 1:1
   admission is known.
3. **Reset not observable.** Whether a native frame encoded `reset = true` cannot be
   seen from outside. The review must infer it from selection, `has_previous`, epoch
   and content size. A test-visible flag would make the reset checks exact.
4. **Single edge-policy source.** Should `metalfx/composite.wgsl` and
   `upscaling/resolve.wgsl` share one edge/coverage function, so the 0072 fixes apply
   to both paths by construction?

This session made no memory writes, read no credentials, used no accounts, did not
push and opened no PR. It also requested or implied no phase approval.

---

## Correction session — helper acceptance before the supervised build (2026-10-13)

**Status: both helper defects are closed. This is still preparation only.** I ran
no Cargo and used no target, GPU or screenshot. The five cases and the plan of 119
submissions and 18 PNGs are unchanged. Everything above this section, including
the prepared result, is preserved; this section is appended.

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. No
  subagents were used.
- **ACP proof:** line 2 of root's ACP log
  `~/.codex/incant-evidence/claude-0074-helper-repair-acp.log` reads "Claude session
  verified: Opus 5.5; thinking: max". Its SHA-256 was `2509b46a…` when I read it.
- **Brief:** SHA-256 `6443f184e0fdafd7…`, revision committed by root in
  `fe4ff9ca`. It did not change during this session.
- **Announced source** (read-only): commit `535003d766bcc1386fd2d2f454c83808a0fcc4c8`,
  tree `e98b24847c28f3491f56e38edb051e018748eff7`; freeze
  `temporal-metalfx-union-001/committed-build-source.json`, SHA-256 `bd8295b0…`.

### Defect 1 fixed: the analyzer accepted incomplete evidence

`check` and `publish` now share one `validate()`. Evidence counts as complete only
when all of the following hold:

- `stop.json` is absent, no build, listing or review child failed, and there is no
  `*.failed.json`;
- the source is the granted commit with exact freeze coverage;
- the frozen review binary still has its hash, was built with the granted features,
  and was listed once with exactly the plan's cases;
- the five plan cases ran exactly once, in order, bound to that binary and grant;
- every case has exactly the plan's required checks, all true (the 35 check names
  are now listed per case in `plan.json`), with the exact planned frames, UI
  submissions and PNG names, and the Metal backend;
- logs, reports, events and PNGs match the hashes bound when each child ran;
- the debits are exactly those five runs' charges, so there is no orphan;
- the GPU latch and the target lease are cleanly released and bound to the final
  ledger, budget and grant.

**`check`** always writes a new summary containing the problems and a SHA-256 binding
of every file a pass depends on, then exits nonzero on any problem.

**`publish`** independently validates again, and also requires the latest summary to
be a pass with the same grant and an identical binding. 0073's analyzer served only
as a read-only reference; this one is 256 lines with no framework.

**Controls** (`test_analyze.py`): 10 of 10 pass.
- Complete synthetic evidence checks and publishes.
- Each of these refuses: empty evidence, a missing case, a false check, a failed
  child, a changed PNG, a changed report, an orphan debit, a stale summary, and a
  failed summary.
- `check` retains a failing summary each time.

### Defect 2 fixed: lint, lease holder and source pinning

- **`build.py lint`:** on failure it keeps `lint.json`, latches `stop.json` and exits
  nonzero. Control: a fake `tools/cargo` that exits 101.
- **`lease.py`:** the lease records the grant's root-assigned `holder` and refuses a
  grant without one. `holder` is now a required grant field in the plan.
- **Source pinning (`build.py checkout`):**
  - It accepts only root's `{commit, files}` freeze, and the freeze must name the
    granted commit. Rows need unique relative paths, a 64-hex SHA-256 and optional
    bytes.
  - Any other freeze shape is refused, and nothing is allowed for older producers.
  - After export it requires exact coverage of the plan's build-input roots, the
    same rule as 0073's `freeze_coverage`. The paths must match, every row (the
    probe included) must exist in the commit, and every SHA-256 and size must match.
  - Anything else latches stop. `review` also refuses without exact coverage.
- **Check against root's real pins** (selfcheck, read-only git objects): all pass.
  - The freeze hash matches, its shape is accepted, and the tree matches.
  - All 2448 build inputs are covered exactly.
  - The single row outside the roots is `tools/probes/cooked-temporal-compile.py`,
    which is in the commit.
  - All 2449 blob SHA-256 values match.

### Fixture lint items

- **`chunks_exact(4)`:** it is not in this fixture. Clippy flagged it in 0073's own
  distinct-colour helper (`upscaling_review.rs:493`), which was never copied here,
  so nothing needed to change.
- **Unused `mut`:** the only instance in this lineage is the `probe` case's
  `let mut report` (0072 line 1418, 0073 line 1336), and this fixture removed
  `probe`.
  - Apart from those two items, the reused helpers passed both earlier
    `clippy -D warnings` runs.
  - I reviewed the new code's `let mut` bindings by inspection.
- **One defensive change, semantically identical:** `outside()` now reads
  `!(columns.contains(&x) && rows.contains(&y))`, because clippy's `nonminimal_bool`
  can flag the `!a || !b` form. No lint was disabled, and no look, policy or
  observation definition changed.
- **Against `535003d7`:**
  - The public API the fixture uses is unchanged, and the jitter table equals the
    fixture's copy.
  - The corrected portable history (partial-mass reprojection) and the unjittered
    overlay composite do not touch the current-only spatial rule, so the
    current-only boundary observation still holds.
- **Still not compiled and not clippy-checked:** root's foreground build comes next.

### Verification (all CPU-only)

| Command | Result |
|---|---|
| `selfcheck.py --write …/preparation/selfcheck.json` | exit 0, `all_pass`; adds per-case check names, announced-source coverage and the jitter check against `535003d7` |
| `test_lifecycle.py --receipt …/preparation/lifecycle.json` | 24/24: 19 earlier tests plus 5 new controls (lint, holder, freeze shape, coverage, checkout refusing before export) |
| `test_analyze.py --receipt …/preparation/analyze.json` | 10/10 |
| In-memory mutation check (not committed) | 13 of 13 disabled guards were caught by their controls; every control passes unmutated |

The receipts in `preparation/` were regenerated for the corrected files. The
originals remain in commit `3354cdfd`.

### Changed paths

- **Edited:**
  - `visual/tools/{analyze,build,lease,selfcheck,test_lifecycle}.py`;
  - `visual/plan.json` (build inputs, per-case checks, `holder`, freeze shape and
    run order);
  - one expression in `visual/review/metalfx_visual_review.rs`;
  - this appended section.
- **Added:** `visual/tools/test_analyze.py`.
- **Regenerated:** `visual/preparation/{selfcheck,lifecycle,analyze}.json`.
- **Unchanged:** `brief.md` and `tools/numeric.py`.
- **Transients:** only the Python temporary directories the tests created and
  removed. No `/tmp` files, and nothing written to `~/.codex/incant-evidence`.

### Next, for root

Run the foreground supervised build, then issue the one-shot grant. It needs:
- `holder`;
- `source_commit` `535003d7…` and `source_tree` `e98b2484…`;
- `source_freeze` `{path, sha256: bd8295b0…}`;
- `features`;
- `visual_tree` of the committed `visual/` directory;
- an allowed target and a prior release pin.

`analyze.py check` now runs after both releases.

This session made no memory writes, read no credentials, used no accounts, did not
push and opened no PR. It requested or implied no phase approval, and leaves no
active children or background tasks.

---

## Execution session — cooked native MetalFX visual review (2026-10-13)

**Status: executed once under root's grant. The numerical and contract verdict is
PASS. The appearance verdict is recorded separately in
`visual-execution-result.md`.** The GPU and the target are both released. This
session made no source, plan, threshold or helper edits and ran no retries.

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
  Line 2 of root's ACP log `claude-0074-cooked-execute-acp.log` reads "Claude
  session verified: Opus 5.5; thinking: max". No subagents were used.
- **Verified before device work:**
  - brief `7a0b9162…`;
  - grant `40e64a7b…`, whose visual tree `b2c3e14e…` equals HEAD's;
  - root build receipt `ba6f2a06…`, build record `f88a64e4…` and source record
    `1d52c532…`;
  - frozen binary `e00b947a…`, with exact coverage of 2448 build inputs plus the
    probe;
  - 0072's GPU release `3c54be77…`, with no live children;
  - the global GPU lock was free.
- **Run** (Apple M5 Pro, Metal, macOS 27.0.1, all serial and in the foreground):
  - `list review`, then `stationary`, `motion`, `excluded`, `transitions` and
    `viewport`. All exited 0, and all 35 required checks are true.
  - Exactly 108 frames, 11 UI submissions and 18 PNGs were charged.
  - `gpu.py release` (08:52:35Z) and `lease.py release` (08:52:39Z) were both
    clean.
  - `analyze.py check` passed, and `analyze.py publish` re-validated and then
    published.
- **Appearance verdict:**
  - Native MetalFX at content scale 1 shows no native-specific defect. Jitter is
    undone (fitted slopes are effectively zero), there's no disocclusion ghost,
    overlays and UI are excluded and byte-stable, viewport placement is exact, and
    resets are clean.
  - It matches or beats the same-source portable arm.
  - It is still not accepted as finished anti-aliasing. Silhouette shimmer remains
    in both arms because of the engine's current-coverage boundary policy.
  - The portable upscale failure at 12/16 and 8/16 is retained, not relabelled.
- **Selections:**
  - native `MetalFx` at 16/16 in every case;
  - `Portable(Some(Scale))` at every 12/16 and 8/16 step, so native upscaling was
    never selected;
  - `Portable(Some(FractionalViewport))` at 10/16 and for the fractional viewport.
- **Request to root:** before any native-upscaling review I need the raw MetalFX
  supported content-scale values for this device. The engine-admitted range
  excludes 0.75 and 0.5, and an upscale-factor versus content-scale interpretation
  mismatch would fit that, but it is unverified.
- **Screenshots:** `visual-screenshots/` holds 18 published PNGs at 960×540, all
  personally inspected, plus a manifest bound to the grant and the passing
  summary.
- **Evidence:** `visual-evidence/` holds 32 records copied unchanged with a
  manifest, and the five case stdout logs as deterministic gzip.
- **Limitations:**
  - MetalFX was observed only at scale 1.
  - Reset and range facts are inferred.
  - One device, offscreen only.
  - No iOS, phone, performance, timing or memory claims, and no phase approval.
- **Transients and process state:**
  - `/tmp/0074-lease-release-unused.txt` held the lease-release output and was
    deleted.
  - The derived crops and difference maps are in the evidence directory's
    `inspection/`, not published.
  - No memory writes, credentials, accounts, pushes, PRs, desktop capture, native
    window or audio.
  - No active children or background tasks remain.

## Corrected-scale preparation session — second frozen execution (2026-10-13)

**Status: preparation only, complete. Nothing was granted, built or run.** The
details are in `corrected-scale-preparation.md`, and the focused helper diff is
`corrected-scale-helper.diff`. All earlier sections of this file and all
first-execution evidence are unchanged.

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. Line
  2 of root's ACP log `claude-0074-scale-preparation-acp.log` reads "Claude session
  verified: Opus 5.5; thinking: max". No subagents were used.
- **Root input verified:** acceptance `8708082e…`, scale query `e5610fe0…` (Apple M5
  Pro, raw [1, 3], 0 submissions), and corrected source `077f7c39` (tree `408c4bb2…`;
  the only build-input changes from `535003d7` are the three `metalfx` scale-unit
  files).
- **Changes:**
  - New namespace `0074-metalfx-visual-002` and publication `visual-screenshots-002`.
  - 535003d7 and its freeze `bd8295b0…` are refused as superseded.
  - The 12/16 and 8/16 native steps (transitions d12/d8, viewport framed-d8) now
    require `Selection::MetalFx`; `NativeOrScale` is removed.
  - Cases, 119 submissions, 18 PNGs at 960x540, checks, tolerances, content, crops,
    sample counts and the grant/freeze protocol are unchanged.
- **CPU checks:**
  - `selfcheck-002.json`: `all_pass` true, `5f2b6dff…`. It includes the strict-native
    replay and the negative control on the first fixture.
  - `focused-tests-002.json`: 7/7, `91066636…`.
  - The one-off in-memory mutation controls all detected their mutation.
  - The full suites were not rerun, and no Cargo or GPU work was done.
- **Focused test runner,** exactly as run from the worktree root (exit 0). It runs
  three analyzer tests and four lifecycle tests in one suite, writes the receipt
  with exclusive create, and exits nonzero on any failure:

  ```
  /opt/homebrew/bin/python3 -I -B -c '
  import json, sys, time, unittest
  sys.dont_write_bytecode = True
  sys.path.insert(0, sys.argv[1])
  import test_analyze as a, test_lifecycle as l
  names = [(a.Analyze, n) for n in ("test_complete_evidence_checks_and_publishes", "test_false_check_refuses",
                                    "test_scale_fallback_selection_refuses")] + [
          (l.Lifecycle, n) for n in ("test_superseded_source_refused_before_checkout", "test_ill_formed_sources_refused",
                                     "test_explicit_source_passes_the_grant_gate",
                                     "test_checkout_refuses_a_freeze_naming_another_commit_before_export")]
  started = time.time()
  result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(cls(n) for cls, n in names))
  with open(sys.argv[2], "x") as out:
      json.dump({"cpu_only": True, "focused": True, "passed": result.wasSuccessful(), "tests_run": result.testsRun,
                 "tests": [f"{cls.__name__}.{n}" for cls, n in names],
                 "failed": [str(t) for t, _ in result.failures + result.errors],
                 "seconds": round(time.time() - started, 2), "python": sys.version.split()[0],
                 "records": a.RECORDS + l.RECORDS}, out, indent=1)
      out.write("\n")
  raise SystemExit(0 if result.wasSuccessful() else 1)
  ' handoffs/0074-metalfx-review/visual/tools handoffs/0074-metalfx-review/visual/preparation/focused-tests-002.json
  ```
- **Open question for root:** 077f7c39 resets native history on every content-size
  change, so the retained `transition-d12-f7.png` and `transition-d8-f10.png` will
  be post-reset frames. The 75%/50% appearance verdict will therefore not show
  converged native quality. Should a later revision retain d12-f9 and d8-f12, or
  lengthen these steps within the reserve? I left frame retention unchanged, as the
  brief requires.
- **Unchanged limitation:** the silhouette-policy defect is still open and is not
  fixed by the unit correction.
- **Screenshots:** none in this session.
- **Process state:** no transient files, background processes, memory writes,
  publication, push or PR. Root's `brief.md` is left uncommitted.

## Corrected-scale execution session — native MetalFX review 002 (2026-10-13)

**Status: executed once under root's grant. Numerical and contract verdict: PASS.
Appearance is judged separately in `corrected-scale-execution.md`.** The GPU and the
target are both released. I made no build, lint, fixture, source, helper or grant
changes, and ran no retries.

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
  Line 2 of root's ACP log `claude-0074-corrected-scale-gpu-acp.log` reads "Claude
  session verified: Opus 5.5; thinking: max". No subagents were used.
- **Verified before device work:**
  - grant `fe6bfa07…` (the brief's authority SHA), with visual tree `0bb15f5c…`
    equal to HEAD's;
  - source `077f7c39`, freeze `62db43cb…`, binary `059d28ac…`;
  - root review `9cbed8d0…` (accepted; its 5 pinned records unchanged);
  - the 0069 release `55ee54d0…` (09:29:06Z, 18 children, nothing alive);
  - the lease held under the same grant, no stop or release latch, and a free GPU
    lock.
- **Run (09:42:11–09:42:49Z):**
  - `list review`, then stationary, motion, excluded, transitions and viewport,
    once each, all exit 0.
  - All 35 checks are true. The budget was exactly 108 frames + 11 UI = 119
    submissions and 18 PNGs.
  - The brief's parenthetical case list differs from the plan order; I ran the plan
    order, which the helpers enforce.
- **Releases:** `gpu.py release` at 09:42:53Z (no owned processes, nothing alive,
  not stopped), then `lease.py release` at 09:42:57Z (no processes on the target).
- **Analyzer:** `analyze.py check` passed (`c29186f5…`, 0 problems). `analyze.py
  publish` wrote `visual-screenshots-002/` (manifest `6bdea57b…`).
- **Selections:**
  - `MetalFx` at 12/16 (720×405) and 8/16 (480×270) in transitions, and at 8/16
    in the viewport. This is the first native upscaling in this packet.
  - `Portable(Some(FractionalViewport))` at 10/16 and in the fractional viewport.
  - Native resets were inferred at a-1, cut-5, d12-7, d8-10 and full-15.
- **Appearance (my own inspection of all 18 PNGs and derived crops):**
  - The full-resolution controls are unchanged. The off and portable PNGs are
    byte-identical to the first execution, and the native PNGs differ only
    slightly, on edges.
  - At 75% and 50%, the retained native frames are reset frames. This run therefore
    validates actual selection and initial quality, not converged quality.
  - Silhouettes and the one-texel grille step and fragment at the input pitch. They
    look like the portable path there, because the current-coverage boundary
    policy feeds them from the current frame in both paths.
  - One native-specific observation: a faint vertical striation (≈ ±1 code, period
    5–6 px) on a flat face in the 8/16 reset frame.
  - The silhouette policy remains an open engine defect and is not fixed by the
    unit correction.
- **Request to root:** a separately reviewed late-frame capture at 75%/50%, for
  example retaining d12-f9 and d8-f12 or lengthening those steps within the
  reserve. It would judge converged native quality and whether the 8/16 striation
  persists. No source fix is indicated by this run.
- **Screenshots:** `visual-screenshots-002/`, 18 PNGs at 960×540, all inspected
  personally.
- **Evidence:** `visual-evidence-002/` holds 33 records copied losslessly, and its
  manifest `d693bd6c…` lists the pins, 18 hash-only files and 10 inspection aids.
  The aids live in the evidence directory's `inspection/` and are not published.
- **Limitations:** one device, offscreen only. There are no phone, performance,
  timing or memory claims, and no phase approval.
- **Transients and process state:**
  - Two `/tmp` stdout captures and `/tmp/0074-002-inspect/` were deleted. The
    helper is retained in `inspection/png.py`.
  - No memory writes, credentials, accounts, pushes, PRs, desktop capture, native
    window or audio.
  - Root's `brief.md` is left uncommitted.
  - No active children or background tasks remain.
