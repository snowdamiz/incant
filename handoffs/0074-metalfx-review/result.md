# 0074 result — bounded native MetalFX ownership execution

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
