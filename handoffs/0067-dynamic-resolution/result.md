# 0067 result — retained dynamic-resolution execution and appearance

## Status

Executed under the root grant and released. All 89 existing GPU regression
tests pass. Every numeric contract in the brief now has passing real-GPU
evidence. Three of the nine numeric runs were preceded or followed by test
defects of my own, and the record shows them rather than hiding them:
- The first fixed-scale run is a retained failure, caused by a defect in my
  test.
- The first natural-cadence run delivered no samples at all.
- A later re-check and a paced run, funded within the 48-attempt cap, cover
  both.

The appearance review is a separate verdict, given below. The GPU slot and
the Cargo target are released, and no owned process remains.

This packet approves no gate. It makes no reference-phone frame-time or
memory claim, no Vulkan/DX12 claim, and no claim for temporal reconstruction,
MetalFX, browser timing or default enablement.

- Model: Claude Opus 5.5 (`claude-opus-5-5`), reasoning effort **max**, as
  reported by this ACP session's runtime. Root keeps the ACP transport log
  (`~/.codex/incant-evidence/claude-0067-acp.log`); I did not read it.
- Platform: macOS 27.0.1 (26A434), Mac17,9, Apple M5 Pro, 48 GiB.
- Adapter: "Apple M5 Pro", backend Metal, IntegratedGpu.
- Profile: `ci` (opt-level 1). These are functional runs, not shipping or
  device measurements.

## Grant, identities and budgets

| Item | Value |
|---|---|
| Grant | `review-ready.json` sha256 `c7e00ed2…`: packet 0067, source `bdd1b205`, target `~/.codex/incant-core-presentation-target` |
| Prior release (0066), verified | `gpu-released.json` `b5d56b5a…`, `target-lease.json` `c5953f6d…`, `gpu-ledger.jsonl` `5946821d…`; no surviving 0066 process |
| Source | HEAD `bdd1b205`; 14/14 manifest sources match the commit and worktree (manifest `4a0b9090…`); source review `3be91d75…` accepted; WGSL and `Cargo.lock` unchanged since base `d5702546`; brief `2669005d…` (unchanged during the session) |
| Toolchain | cargo/rustc 1.99.0 (b940084d7 2026-09-28), LLVM 23.1.1; `./tools/cargo --locked --offline --profile ci -j2`; explicit minimal environment; receipts record only allowlisted settings |
| Regression binaries | lib `9730b93d…`, model_gpu `a848f159…`, gpu_host-lib `8438dfbf…`, game `6c115e14…`, ui `d0655f44…` |
| Features | `incant_render` and `incant_render_gpu_host` both `default,physics-deterministic` (no cooked-* shaders), identical in the regression and helper builds; all 313 shared packages have identical feature sets |
| Packet helper | `helper` (drs_gpu `ed2a858b…`, drs_appearance `bef3671a…`) → `helper-r2` (`52fe5020…`) → `helper-r3` (`3fe87919…`). Only packet test code changed between builds; the reasons are recorded in the `tools/cases.json` amendments |
| Lease / GPU | lease acquired 03:53:58Z; GPU latch written 04:10:29Z (24 children, ledger `cc7ebbec…`); target released 04:13:42Z, nothing deleted |
| Budgets | numeric frame attempts **48 / 48** (44 reached the queue); appearance renders **12 / 24**; every frame ≤ 960×540. Regression suites are counted separately |

## Regression suites (existing tests, serial, `--ignored --test-threads=1`)

| Binary | Tests listed | Ignored (GPU) | Result |
|---|---|---|---|
| incant_render lib | 102 | 32 | 32 passed, 0 failed (2.1 s) |
| model_gpu | 50 | 50 | 50 passed, 0 failed (45.4 s) |
| incant_render_gpu_host lib | 3 | 0 | no ignored cases (CPU mailbox tests); listed only |
| gpu_host game | 5 | 5 | 5 passed, 0 failed |
| gpu_host ui | 2 | 2 | 2 passed, 0 failed |

Total: **89 / 89** ignored GPU cases pass, the same count as the reviewed
base. No case was skipped or refused.

## Numeric verdict (new focused GPU tests)

Cases live in `tools/drs/tests/drs_gpu.rs` and use only public engine APIs.
Raw receipts are in `evidence/gpu-logs/case-*.stdout.txt.gz`. Gzip keeps
libtest's trailing blank line byte-exact; `evidence/manifest.json` gives the
raw and retained hashes.
- The **witness** is a test-only extra `Queue::on_submitted_work_done` callback,
  registered in the provider's `present()` immediately before the consumer
  registers its own. Both fire in the same dispatch batch, so the witness
  bounds when the host's callback ran.
- **Injections** are explicitly identified and happen only at the provider
  boundary: acquisition/presentation refusal, a blocking poll inside
  `acquire`, and a queued buffer-copy backlog in `present`.

1. **Fixed controls, composed host with UI, 959×539, offset fractional
   viewport (`[37.5, 21.25, 880.25, 495.5]`, rounded chrome), perspective.**
   - Receipts report scale 16, 12 and 8 sixteenths with internal sizes
     959×539, 720×405 and 480×270 (`ceil(output×n/16)`), while the output
     stays 959×539.
   - Disabled and fixed 100% are **byte-identical**.
   - Chrome outside the viewport is identical at every scale (79,967 px each).
   - UI layout boxes are identical across all four.
   - A pointer press/release at the physical Resume centre focuses and
     activates Resume once at both 100% (disabled) and 50%.
   - Unchanged-size frames created no pipelines or textures (wgpu registry).
   - **Run 1 failed five checks because of my test defect:** the UI exclusion
     and opaque-interior boxes were in logical UI units. With a camera
     viewport, the existing UI contract maps the logical UI viewport onto the
     viewport rect (offset 37.5/21.25, scale 880.25/959), which
     `a1-offset-ui-disabled.png` shows. The run is retained unchanged.
   - **Re-check** (`fixed_scales_reference_recheck`, 2 attempts) used the
     appearance run's retained frames, whose RGBA hashes equal run 1's
     disabled, 12/16 and 8/16 receipts. It rendered only two new disabled
     internal-size references, using physical UI boxes:
     - 12/16: **0 of 341,989** world pixels differ from the nearest-texel
       expansion of the reference (`texel = floor((2x+1)·n/32)`).
     - 8/16: **0 of 342,858** differ.
     - Opaque UI control interiors equal the disabled control at both scales
       (0 of 39,380).
   - Not re-checked: run 1's tick-2 focused-control pixel comparison, for
     lack of budget. Tick-2 layout, focus and activation did pass.
2. **Orthographic, lower-level owner contract**
   (`RetainedScene::set_dynamic_resolution` / `record_frame_time`, with the
   test's own completion callbacks).
   - A 12/16 frame under the offset viewport matches its internal-size
     reference exactly (**0 of 432,825**).
   - Telemetry results follow the typed contract:
     - foreign receipt (owner replaced) → `Ok(false)`;
     - NaN, 0, −1 and 10000.1 ms → `Err(Telemetry)`, with state unchanged;
     - valid → `Ok(true)` (8→9);
     - duplicate → `Ok(false)`;
     - previous extent epoch → `Ok(false)`;
     - valid current epoch → `Ok(true)` (9→10);
     - previous scale → `Ok(false)`.
   - Refused staging (changed bindings), planning (viewport outside the
     target) and encoding (Rgba16Float) each returned an error and left
     receipt 4 current. The next accepted frame is sequence 5 at 10/16.
   - Real lower-level latencies, from before submit to callback dispatched by
     an explicit poll: 1.66, 2.88, 1.55, 1.54 and 1.71 ms.
3. **Pixel-perfect camera** (logical 240×135, ppu 32, offset viewport).
   - With an adaptive policy at 4/16, the frame is **byte-identical** to the
     disabled control.
   - Receipt: pixel-perfect, 16/16, internal 240×135, output 959×539.
   - Every logical pixel is a uniform 3×3 block at the contract-derived
     letterbox (117, 66).
   - The completed pixel frame delivered no feedback (0 discards; scale
     unaffected).
4. **Delayed delivery across resize and scene replacement** (stepping
   policy, so every accepted sample raises the scale by 1/16).
   - Frame 1's completion was held behind the injected backlog and dispatched
     after tick 2's take (phase 22, 37.6 ms).
   - The resized extent 800×450 kept 4/16. The old-extent result was
     **discarded** (1), and the new epoch's own sample was accepted (→5/16).
   - A Space-key `despawn` made the next sync a scene replacement with a
     fresh owner (sequence 1, 4/16). The previous owner's late result was
     **discarded as foreign** (2), and the replacement's own sample was
     accepted (→5/16).
   - Frames that had no consumer callback still completed (witnessed).
5. **Drop and recreate.**
   - Host A was dropped with its callback outstanding; that callback fired
     32 ms later during host B's tick.
   - B started fresh (sequence 1, 4/16, 0 discards). Only B's own delivery
     reached it (→5/16, 0 discards).
6. **Terminal failures** (direct `RenderInputConsumer::accept`).
   - For both acquisition and presentation refusal, the consumer stayed
     failed (accepted 1, no receipt, 0 discards).
   - The earlier frame's callback dispatched only after the failure and
     changed nothing.
   - The refused presentation's frame was submitted and completed (target
     fully overwritten) but not counted: presents = 1.
7. **Adaptive runs with the realistic policy** (16.667 ms ±10%, recovery 2,
   cooldown 1, slew 2, range 8–16).
   - *Natural cadence* (7 back-to-back ticks, about 0.9 ms each): no
     completion was dispatched by the host's own non-blocking polls before
     the run ended, so the scale correctly stayed at 12/16 with no samples.
     The first frame of a process needs about 6.6–6.9 ms to complete,
     longer than the whole 6.3 ms run.
   - *Paced* (one explicit poll after each tick, no sleeps): the delivered
     latency bounds were [6.61, 6.88], [1.52, 1.73] and [2.74, 2.88] ms, all
     under budget.
   - Observed receipts were **12, 12, 14, 14**: a two-sample streak, then a
     +2 slew to 840×472, then a cooldown-consumed sample.
   - An in-test Rust replay and an independent Python replay both predict
     exactly this sequence, with no ambiguous interval.
   - Texture and pipeline counts stayed constant: the replaced attachment
     pair was freed.

Latency here means completion latency: CPU preparation, queue backlog,
presentation blocking and callback-dispatch delay. It is not GPU execution
time, a refresh interval, whole-game frame time or a benchmark.

Offline (`tools/analyze.py`):
- All six reduced-scale appearance frames are internally consistent with
  nearest-texel expansion (0 pixels differ within any texel group).
- Every retained PNG decodes to the helper's readback hash.

## Appearance verdict (separate)

Reviewed unmasked, complete frames and 2–4× nearest-zoom crops.
- **Aspect, placement, clipping and chrome:** correct in every set. There is
  no stretch, the camera aspect is unchanged at odd extents and in
  orthographic projection, the rounded viewport chrome and canvas are
  untouched, and nothing leaks outside the viewport.
- **UI:** stays at physical resolution and is identical at every scale.
  Labels sit crisp over the scaled world, and opaque controls hide the scaled
  pillar completely.
- **100%:** identical to disabled.
- **75%:** irregular 1- and 2-pixel stair steps on slanted edges (crate, floor
  line) and uneven stroke widths in world text, the expected signature of a
  non-integer nearest expansion. Mild at this size, but likely to shimmer in
  motion (not observed; static frames only).
- **50%:** regular 2×2 blockiness. It is clearly visible on world text, on
  the shallow floor edge (already aliased at 100%, since the scene has no
  MSAA) and on cube edges. The text remains legible.

This is the deliberate nearest-texel fallback, not temporal upscaling or
MetalFX. It is acceptable as a fallback, and visibly degraded at 50%.

## Screenshots and figures (unedited captures; crops are derived)

- `evidence/appearance/a1–a4` — offset viewport with UI: disabled, 100, 75, 50.
- `evidence/appearance/b1–b3` — full 960×540 with UI: 100, 75, 50.
- `evidence/appearance/c1–c3` — orthographic: 100, 75, 50.
- `evidence/appearance/d1–d2` — pixel camera: disabled, fixed 50% (identical).
- `evidence/figures/*.png` — zoomed crops: world text, crate/floor, left floor
  edge, UI over the pillar, chrome corner, orthographic, pixel bypass. Panel
  order is in `figures.json`.
- `evidence/recheck/reference-{12,8}.png` — the disabled internal-size
  references.

## Changed paths (all under `handoffs/0067-dynamic-resolution/`)

- `tools/`: `common.py`, `lease.py`, `build.py`, `gpu.py`, `analyze.py`,
  `figures.py`, `retain.py`, `pngio.py`. These are adapted from 0066's
  environment-safe tools; the 0065 logger is not used.
- `tools/cases.json`: the frozen ledger, with two recorded amendments.
- `tools/appearance.json`: the frozen plan.
- `tools/drs/`: a packet-local crate with its own workspace and a pruned
  `Cargo.lock` verified against the repository lock. It holds `src/` fixture,
  probe, pixel and helper code, `tests/drs_gpu.rs` and
  `src/bin/drs_appearance.rs`.
- `requests/observer-seam.md`: a precise `cfg(test)` observer request for
  Astra.
- `evidence/` (106 files, 1.3 MB, `manifest.json` with raw and retained
  hashes):
  - ledger, latch and lease, and the build identities with compiler-artifact
    features;
  - raw logs for every child;
  - analysis, appearance captures, figures and references;
  - `executed-source/helper-r3-sources.tar.gz`, the exact sources that ran.
- No engine, CI, main, dependency or root-written file changed. After
  execution I applied only rustfmt (whitespace) and one clippy fix
  (`map_entry`, behaviour-equivalent) to the helper. `cargo fmt --check` and
  `cargo clippy --all-targets -D warnings` now pass.

## Commands (from the worktree root; `python3` = `/opt/homebrew/bin/python3 -I -B`)

`tools/lease.py acquire`;
`tools/build.py freeze|tests|helper-lock|helper|helper-r2|helper-r3`;
`tools/gpu.py list <5 suites>`, `suite lib|model_gpu|gpu_host-game|gpu_host-ui`,
`case <9 runs>`, `appearance`, `release` (later refusals verified);
`./tools/cargo fmt|clippy --manifest-path handoffs/0067-dynamic-resolution/tools/drs/Cargo.toml`;
`tools/analyze.py`, `tools/figures.py`, `tools/retain.py`; `tools/lease.py release`.

## Limitations and unavailable evidence

- Only Metal on an Apple M5 Pro was tested: no Vulkan/DX12, no reference
  phone, no AWS Device Farm and no windowed or vsync host. This is not a
  performance measurement.
- The host's exact delivered milliseconds and discard reasons are not
  publicly observable. Witness intervals bound them (see
  `requests/observer-seam.md`).
- The native ticket destructor/reservation interleaving was not forced; the
  CPU regression covers it.
- Only static frames were reviewed: no motion or shimmer review. Only the
  default feature set was used (no cooked-output or cooked-geometry variants).
- Browser timing remains unsupported and untested.
- Unchanged-frame reuse is shown by wgpu registry counts for pipelines and
  textures only. There is no claim that driver allocations or frame encoding
  are allocation-free.

## Open questions for Astra

1. Should the composed-route cases be integrated into `incant_render_gpu_host`
   with the requested `cfg(test)` observer?
2. In unpaced loops the controller receives no feedback until a poll follows
   GPU completion. Is that acceptable for the hosts that will opt in?
3. 75% gives irregular nearest steps. Is a policy or quality decision
   (owned by Astra and the director) wanted before any host enables DRS?
