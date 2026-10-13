# 0069 Core result presentation: performance and runner corrections

## Status

**Prepared for root's source review. Nothing executed. No visual acceptance claimed.**

- No Cargo, build, shared target, GPU, capture, native window or device audio.
- No grant exists, and I neither polled nor waited for one.
- No memory, auto-memory, external notes or other checkouts were written.
- Everything is uncommitted on HEAD `12860367` (root's brief commit), which
  keeps `f5eba1d7`.
- `result.md` (the previous delta) is untouched. Its byte-identical copy is
  `previous-preparation-result.md` (`e4738954…`). This file is the current
  result.

## Model and effort

Claude Opus 5.5 (`claude-opus-5-5`) with thinking effort **max**, as this
session's context reports. Root's ACP record is the verification.

## Inputs read

| Input | SHA-256 | Notes |
| --- | --- | --- |
| `brief.md` | `52e9108c…` | Priority revision "complete performance and runner corrections before any grant". Rechecked at the end: unchanged. |
| `~/.codex/incant-reviews/core0069-prep-second-20261013.json` | `62c21328…` | Findings `perf`, `global-lock`, `release-race`, `release-ledger`, plus `regression_budget`. |
| `root-helper-review.json` | `456302ca…` | Root-owned, unedited. |

## Acknowledged feedback

Root is right about the cost. The draft halo took 25 texture reads per filter tap,
so up to 400 reads per fragment on every label. I kept the visual direction:

- the same opaque `#0A0C10` halo, 1/16 em within 1–2.5 px;
- the same layering, under each glyph;
- the same clip idea.

The algorithm changed openly, not silently. The halo is now coverage prepared with
the glyph rasters and drawn as an ordinary glyph.

## Finding closure

| Finding | Change | Checks (written, not run unless noted) |
| --- | --- | --- |
| `perf` | Halo coverage is dilated on the CPU when a paint is built and stored as its own atlas entry. The renderer draws it as one glyph quad, so it costs one glyph's taps. The glyph's own entry is byte-identical, and `ui.wgsl` is byte-identical to main (`f67e4982…`). | `incant_text` unit and atlas tests, `incant_ui` paint test, renderer CPU, GPU and mutation tests (below). No timing measured and no phone claim. |
| `global-lock` | GPU and adapter children first take `~/.codex/incant-evidence/native-gpu.lock`, waiting at most 600 s, then the packet lock. Both are held across the debit, launch, kill/reap and exit accounting, and released in reverse. CPU children don't take it. | 3 synthetic controls, **run: pass**. |
| `release-race` | `release` takes the packet lock before any check and keeps it until the latch is durable. It rechecks recorded child groups and target users under the lock, and refuses a partial earlier release. | Controls race a GPU run and a lock holder inside the window, **run: pass**. |
| `release-ledger` | `release` appends the ledger's final row first. It then hashes the finished ledger (and its row count) into `release.json`, then writes `released`. Both are create-only and fsynced, and the directory is synced. Every writer refuses once `released` exists. | Control checks the hash still equals the ledger after every writer is refused, **run: pass**. |
| `regression_budget` | Frames are debited to two ceilings: `appearance` 24, which is unchanged and used by `capture.py`, and `regression` 128. `regression.json` pins both test sources by SHA-256 and lists 8 cases, 39 frames, counted from source. `regression.py` runs each case once with no retry. | Controls cover the ceilings, case list and libtest parsing, **run: pass**. |

## Prepared halo

### Data path

1. `UiPaint::prepare(…, halo: HaloRule, …)` rasterizes every glyph into the
   paint's atlas, as before.
2. For each label glyph it also calls `TextEngine::cache_halo` at
   `rule.radius(font_size)`.
3. The game binding passes `incant_render_data::UI_LABEL_HALO`
   (`em 1/16, min 1, max 2.5`). The renderer theme re-exports the same constant
   as `LABEL_HALO_RULE`.
4. `UiSettings` gained no field, so a host cannot choose another rule.

### Atlas entry

- **Key.** The glyph's raster key plus a new `GlyphKey::halo_radius_bits`. It is
  zero for ordinary coverage, so a halo can never alias a glyph.
- **Size.** `ceil(r)` texels larger on every side. The origin moves by the same
  margin, also for an empty outline: that fixes a space-glyph inconsistency found
  in this pass.
- **Limits.** Halo entries count against the atlas's existing byte and entry
  limits, as glyphs do.
- **Refusals.** A radius outside `(0, 4]` is refused (`MAX_HALO_RADIUS`).

### Dilation (`incant_text::halo::dilate`)

- Each halo texel is the largest `clamp(r + c − d, 0, 1)` over glyph texels with
  coverage `c > 0` at centre distance `d`.
- A straight edge through a half-covered texel therefore grows by exactly `r`.
- The halo is never below the glyph's own coverage.
- Only correctly rounded f32 operations are used, so every platform prepares
  identical bytes.

### Renderer (`geometry.rs`)

- One `KIND_GLYPH` quad per inked label glyph from its halo entry, before the
  glyphs, in `LABEL_HALO`, muted when disabled.
- **Clip.** The text clip grown by `ceil(r) + 1`, never past `visible`, and empty
  when the text clip is empty. A glyph the clip cuts keeps up to that much of its
  hidden part's halo. This is now documented.
- **Refused.** A label without halos, a wrong count, halos on another role, a halo
  of another glyph or radius (the radius must equal the theme rule for the
  glyph's size), and a halo entry passed as a glyph.

### Cost (bounded, not measured)

- **GPU.** One more glyph quad per label glyph, with the glyph filter: 1 tap at
  1x, at most 16 when minified.
- **CPU.** Only when a paint is rebuilt: first bind, viewport change, changed
  text or reconciled widgets. A reused paint costs nothing.
  - Core's HUD bindings change on gameplay events, not per frame.
  - Each halo texel reads at most `(2·ceil(r)+1)²` glyph texels (49 at 2.5 px,
    81 at the cap) and stops early at full coverage.
- **Memory.** Halo entries share the paint's atlas: 2048², 8,192 entries for the
  game binding.

### Implications and their tests

| Area | Effect | Documented / tested by |
| --- | --- | --- |
| Cooked data | None. No authored or cooked field. The atlas is runtime-only, and the cooked UI shader, its digest and the 64-byte instance layout are unchanged. | `native-ui-draw.md` "Label halo preparation". The existing cooked-shader digest check still covers `ui.wgsl`. |
| Layout | None: the halo is never measured. | Same doc; `PRESENTATION.md` limitation. |
| Public Rust API | `GlyphKey` gains a field. `UiPaint::prepare` gains a `HaloRule` argument. New: `UiTextPaint::halos`, `UiWidget::halos`, `cache_halo`, `HaloRule`, `MAX_HALO_RADIUS`, `UI_LABEL_HALO`. | All in-tree users updated (`git grep` of `UiWidget {`, `GlyphKey {`, `UiPaint::prepare`, `UiSettings {`). |
| Glyph coverage contract | Unchanged: own entries are byte-identical with or without halos. | `incant_text/tests/atlas.rs`, `incant_ui/tests/fonts.rs` |
| Halo bytes | Real Latin, CJK and Arabic halos at 12/17/22/40 px are recomputed independently in f64 over all glyph texels: within one code, nothing outside the margin. | `ui_tests.rs::prepared_label_halos_are_the_documented_dilation_of_their_glyphs` |
| Renderer contract | Layering, halo quads, the `ceil(r)+1` clip with padding, ancestor, parked and disabled cases, and eight refusals. | `ui_tests.rs` |
| GPU draw | The reference draws the prepared entries. A new "hugged" label, whose text clip equals its ink, makes the growth past the clip observable on GPU. Mutations rejected on CPU: absent halo, undilated halo, halo over the text, halo cut at the text clip. The shader direction-table test was removed with the shader code. | `ui_draw_tests.rs` |
| Every game's labels | Core and Planar labels now draw the halo. Planar's two prose sentences were corrected; its rendered look is unreviewed. | `samples/planar/PRESENTATION.md`, `samples/planar/hud.py` (docstring only) |

## Runner corrections (`tools/`)

- **`lease.py`**
  - `exclusive()` flock helper with a bounded wait.
  - `run(gpu=True)` takes the global lock, then the packet lock, and limits GPU
    children to 120 s.
  - `debit=(category, run, frames)`: positive whole frames, unique run ids across
    categories, per-category ceilings. Only GPU children may debit.
  - The start row records the GPU-lock wait.
  - The ledger fsyncs each row.
  - `write_new` creates files create-only and synced, via a hard link.
  - `release` changed as described above.
  - `cargo test` is allowed only with `--no-run`.
  - `ALLOWED_ENV` adds the two test-image directories.
- **`capture.py`**: every adapter child, layout-only or not, is a GPU child. Frame
  runs debit `appearance`.
- **`regression.py` (new)**
  - `build`: `cargo test --no-run` for both binaries, recorded per freeze
    generation.
  - `run`: rechecks sources, generation and executable digests before each case,
    then runs `EXE --exact TEST --ignored --test-threads=1 --nocapture` from the
    package directory. It uses a scratch HOME and TMPDIR, so no pipeline cache is
    written outside evidence.
  - Pass/fail comes from libtest's single summary, which must show exactly one
    test run.
  - A failing test is recorded and the run continues. A timeout, interruption or
    refusal stops it.
- **`regression.json` (new)** and a short `plans.json` note.
  `build_variant.py` is unchanged: its startup check is a CPU child, because the
  offline package path uses `--cpu-render-input`.

### Regression frame counts, from source

Every Game construction is counted as one frame, so these over-count.

| Case | Frames | Derivation |
| --- | --- | --- |
| `ui::draw_tests::logical_layout_maps_to_physical_pixels_at_integer_and_fractional_scales` | 4 | one `run()` per scale/format, ≤320×180 |
| `ui::draw_tests::nested_clips_cut_actual_glyph_bounds_including_negative_x_rtl_overflow` | 2 | two `run()` |
| `ui::draw_tests::label_halo_separates_text_from_bright_and_dark_backdrops` (new in 0069) | 2 | white and black, 220×50 |
| `ui::draw_tests::ui_composites_over_the_finished_world_frame_and_leaves_uncovered_pixels` | 3 | two world draws, one UI submit |
| `ui::draw_tests::retained_atlas_and_quads_follow_input_identity_and_survive_refused_stages` | 6 | six `run()`; refused stages draw nothing |
| `ui::draw_tests::editor_viewport_region_offsets_scales_and_bounds_the_ui` | 1 | one `run()` |
| `composed_ui_draws_over_the_moving_world_and_reuses_unchanged_resources` (gpu_host `ui`) | 17 | 2 constructions, 8 loop ticks, 7 input ticks |
| `refresh_and_viewport_relayout_use_the_same_ui_without_advancing_the_game` (gpu_host `ui`) | 4 | 1 construction, 1 tick, 2 refreshes |
| **Total** | **39** | ≤ 128 regression; the 24 appearance frames are untouched (22 planned) |

A control checks that the listed tests are exactly the
`#[ignore = "requires a native GPU…"]` functions in the two pinned sources.

## Commands run

All of these were CPU-only and read-only, apart from the tool scratch they created
and removed.

| Command | Result |
| --- | --- |
| `rustfmt 1.99.0 --edition 2024 --check`, then format, on every touched Rust file | clean |
| `python3 -B -m unittest -v test_lease` (tools/) | **19 tests OK**, 9.7 s. Scratch removed, no `__pycache__`. |
| `python3 -I -B lease.py status` | `grant false, lease false, released false, target_users []` |
| `python3 -I -B regression.py run` (refusal self-check) | `refused: no 0069 lease`, exit 3. Nothing written; `~/.codex/incant-evidence/0069` still absent. |
| `ast.parse` of every helper | ok |

The shared `native-gpu.lock` already existed before this session's runs: created
00:59, empty, and untouched since. The controls used a scratch stand-in.

**Not run:**

- any Cargo build, check, clippy or test;
- any Rust unit, CPU or GPU test;
- any capture or regression case.

The Rust changes are reviewed by eye only, including for borrow and type errors.

## Changed paths (SHA-256 at hand-off)

```
b3ad306af479782881ced21bdacdda893c068651d3b46a2e553bac7ac0302813  crates/incant_render/src/ui/geometry.rs
d96bf934007daf6be7fbe8a4f7bf9a3497f420cd286e81814408648a74bfb1ce  crates/incant_render/src/ui/mod.rs
bf4f842c03eccb389c4833f7abfee38574e270ad3c03ce70cbd1c5f51f9d4f24  crates/incant_render/src/ui/theme.rs
fee1ebcf26633a9e8905f7efd42fc844a40f0686f137865506ac11f2613b6169  crates/incant_render/src/ui_draw_tests.rs
1bbc216032eaefe9460ae1ebbe64d6b512d0423959e88dbd3734fabf48c2ca66  crates/incant_render/src/ui_tests.rs
db64cadff721e989f7b49be946020978b2102173693f54c5f45e88892c8b5647  crates/incant_render_data/src/lib.rs
cfcb10b0796899c46e7db0aaaafe3b4c34ff779638b5673c154727503271363a  crates/incant_render_data/src/ui.rs
12c57a930d79bbdaf9728f8274c5e4b9df5b30d7c2d0392beb25064461ac404c  crates/incant_render_game/src/ui.rs
429ba57c69f79a6784c4297dfc1fe76c2c4259a68677b47a55c915ac3389f1be  crates/incant_text/src/atlas.rs
951a3a1ed0b9aec44daf0a71c39568c6d42867267f2717d70dc4a5ba22f1217b  crates/incant_text/src/halo.rs (new)
37c87a4bfd0787db2279073c85fa4994c4047afd182ab742c7e5300a6f04a886  crates/incant_text/src/lib.rs
c644bf379f083794d2611d89c8fedfad85e7e777577e0c83934cccefc19ac108  crates/incant_text/tests/atlas.rs
329d2d63b1917e3cece9ff92fb9949472ae007d10ab116d09a2e8b81a4df686f  crates/incant_ui/src/fonts.rs
4131d10a3fffb17de27de680073b010213dbfb90ccacce05e9cb7e92edfa5649  crates/incant_ui/src/lib.rs
21e420ac3722133a4eaa60316752c3b13ced972b763ed84a124e852ccda01fba  crates/incant_ui/src/paint.rs
a9a94f9e9e832fb94720f63da85616043428ed1aa3b0c126842c7e2944795401  crates/incant_ui/tests/fonts.rs
283960de3795c08caa1244171ef710a64cb94d8c7ff7904da80fce7524833b0f  docs/spikes/native-ui-draw.md
4a54de7dfd55c7afaf4148ce20a243882952f253c39af53aab8872e4c9029ae1  samples/core/PRESENTATION.md
a6019be943514c7163706be399e6675d7aa3b8c7e8933d7107469459fe9fe99a  samples/core/ui/hud.json
0e21d0c1c648b9fa297797e796abc507a8cf35494ca898bee96921e20f1bda91  samples/core/ui/strings.json
054bc3eb51d4fdf86364a0427310b154a264b5232df609e8d1ef53bcfd9b8142  samples/planar/PRESENTATION.md
ca7f7e4a900fc33b8f31c4b500dd378a5682786713455ee436a7e44023dea4c8  samples/planar/hud.py
974101cc51be4af23ffeba5f5485ff91cd933fce1d6cb0836ba95f18e035d8f3  tools/build_variant.py (unchanged this pass)
097b8ce83336c4267fc54ba68d3e5a8c58f7b85171a3abd28b3efcc432a50418  tools/capture.py
bc400af199d76289db71f5b1579e5f2fa97d4aea3450e7d7aa6e1e48794458d8  tools/frame_profile.py (unchanged)
84796818fa99f6c5a20bee693efed8d2450a132360fb3e2544d8b600963f3b1b  tools/lease.py
0f98ab942e5a1ecda2c50c958d8cf35480d9d7e974ae5bf2567c7e34a593b77c  tools/placement.py (unchanged)
3a7fd90e5f5323998ba80e7acdbf93cbf57743c1bd32c85b49d84210b92c4c3c  tools/plans.json
f2063d954650d675456b0f321b6c3482a612c6f4773969c5c5cf98f406b6efff  tools/regression.json (new)
a2fae9634546eed035de1b4d28be0ce82c27ec5c9bc501ed685bc3c8ba7d3270  tools/regression.py (new)
e4dbd020d7d518b74720cf6b50e2ee40767c7dcc139021debaf376936e407acf  tools/result_hud.rs (unchanged)
dba1f4fa352495a4743c71c306fa9fe0011ea362a26b1b878edbde4c43ab1d99  tools/routes.py (unchanged)
183c4610b5226f4d90930bcb45be726b229e7d1a2627312f1bc8403eba1b1ff4  tools/test_lease.py
9fa7fd1d012688e4dec8d0e3983235683e0d04f0acd90d20a25caa4b14d78826  tools/text_width.py (unchanged)
```

`tools/` means `handoffs/0069-core-result-presentation/tools/`. `regression.json`
pins `ui_draw_tests.rs` and `crates/incant_render_gpu_host/tests/ui.rs`
(`7ebe885a…`, unchanged). Any review edit to either file needs a new pin and a
recount.

## Limitations and outstanding review items

**Not run yet**

- Unbuilt Rust: the first compile, clippy and test run happens only after a grant.
- The GPU halo tests and the CPU mutation thresholds (`outside > 20, excess > 8`)
  are estimates until they run.

**Not measured**

- There is no CPU preparation timing, GPU timing or atlas-occupancy measurement.
- Nothing here is phone or mobile evidence. The performance argument is the
  bounded per-texel work, not a measurement.

**Visual changes against earlier work**

- The prepared halo is the texel-domain disc dilation. The previous draft's
  16-direction sampling approximated the same disc. Rendered edges will differ
  slightly, and no pixel equivalence with the earlier draft or the 0065 captures
  is claimed.
- Core labels keep 2 px of side padding and 1 px top and bottom. A deep descender
  whose ink meets its line box can lose the outer part of its halo at the border
  box; the rendered review must check this in every locale.

**Ownership**

- Planar's HUD now draws halos too. Its prose was corrected, but its rendered look
  is unreviewed.
- STATUS and the root-owned evidence documents are not edited.

## Open questions for root

1. Is a 600 s bound on the shared GPU-lock wait acceptable, or should it match
   another packet's bound?
2. The new halo-backdrop GPU test (2 frames, 220×50) is charged to the
   `regression` ceiling as a contract regression check. Root may prefer
   `appearance`, which would leave no appearance reserve (22 + 2 = 24).
3. Should Core's vertical label padding grow before the rendered review, or stay
   until the captures show whether descender halos are cut?


## Revision 1 (2026-10-13): finalized for the CPU-only grant

Root read the prepared halo source (brief `100088d8…`, root commit `c6ce0eb7`),
found no source-correctness blocker and answered the open questions. The
sections above stay as written. Where they disagree with this revision, this
revision supersedes them.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`) with thinking effort
**max**, as this session's context reports. Root's ACP record is the
verification.

### Commits

On `handoff/0069-core-result-presentation`, not pushed:

- **Source and helpers:** `c6a7804ae1f03dd7bfa09a6487ef51ee2f4390e5`, tree
  `64ae439579f3f749fef5d8d822d921e2bf547094`, parent `c6ce0eb7`, 34 paths,
  `Built-by: claude`.
- **Evidence:** the next commit, which adds this file, `result.md` and
  `previous-preparation-result.md`. Root's `root-helper-review.json` stays
  uncommitted.

### Changes in this pass

| Root's answer | Change | Supersedes above |
| --- | --- | --- |
| Non-waiting GPU lock | `exclusive()` no longer waits. A busy global or packet lock refuses at once, with nothing debited, started or written. Start rows record `gpu_lock` (its path, or null for CPU children) instead of a wait time. | "waiting at most 600 s" in the closure table and runner section; open question 1 |
| New halo regression | `regression.json` labels `render-halo-backdrops` as `coverage: new`: coverage this packet adds, not a historical test. The other seven cases are `existing`, present at main196 `3da8b0eb`. Totals are `{existing: 37, new: 2}`, `total_frames` 39, all on the 128-frame regression ceiling. `regression.py` refuses unknown labels or subtotals that disagree. | open question 2; "new in 0069" wording |
| Padding | Core keeps `[2, 1, 2, 1]` for the first bounded visual evaluation. I decide after inspecting the captures. | open question 3 |
| Cross-platform bytes | The claim that every platform prepares identical bytes ("Prepared halo", "Dilation" above) is withdrawn. `halo.rs` and `native-ui-draw.md` now document the bounded f32 arithmetic instead: at most 81 candidate texels, square roots of integers up to 32, one division by 255, a clamp and an 8-bit rounding. They also document the local oracle: a renderer CPU test recomputes real halos in f64 and accepts at most one code of difference. Equality across platforms is unverified. | that bullet |
| Target | `lease.TARGET` is `/tmp/incant-agent-font-target`, which root reserved for 0069's CPU-only grant; the grant must name it. 0072 owns `incant-core-presentation-target`. The new target was not accessed, listed or built. | the earlier target |

### Checks re-run (affected only)

| Check | Result |
| --- | --- |
| `python3 -B -m unittest` on 10 named controls in `tools/` (see below) | **10 OK** in 0.94 s. Scratch removed, no `__pycache__`; `~/.codex/incant-evidence/0069` is still absent. |
| rustfmt 1.99.0 `--edition 2024 --check` on every touched crate | clean |
| Python `ast` and JSON parse of every helper and HUD document; `git diff --check` | clean |

The 10 controls cover:

- run accounting;
- both frame ceilings;
- global lock first, now recorded directly by patching `exclusive`;
- immediate refusal of a busy global or packet lock;
- the release lock window;
- the final ledger hash;
- release refusals;
- the regression list and coverage labels, against the pinned sources and
  main196 (read-only `git show`);
- libtest parsing;
- a grant must name the reserved target.

Not re-run: the supervise/signal, porcelain, identity, freeze and environment
controls. Their code is unchanged, and they passed 19/19 in the previous pass.

### Known gap carried by the source commit

- `samples/core/ui/strings.json` uses 按 and 或 for the zh-Hans prompt.
- By cmap, the committed `CoreSampleSC-Regular.ttf` (from `8b82ad4d`) covers 31
  of the 33 authored Han glyphs and lacks both of these.
- `samples/core/scene.py` refuses to build zh-Hans until the subset is
  regenerated. Regeneration uses `subset_sc.py`, the pinned upstream and the
  fonttools wheel in `provenance.json` and the 0060 cache. It is the first
  granted CPU step, as `result.md` recorded.

### Focused changed-path hashes

Commit `c6a7804a` against `c6ce0eb7`. `[this pass]` marks the 8 paths changed
since the listing above, and `tools/` means
`handoffs/0069-core-result-presentation/tools/`.

```
b3ad306af479782881ced21bdacdda893c068651d3b46a2e553bac7ac0302813  crates/incant_render/src/ui/geometry.rs
d96bf934007daf6be7fbe8a4f7bf9a3497f420cd286e81814408648a74bfb1ce  crates/incant_render/src/ui/mod.rs
bf4f842c03eccb389c4833f7abfee38574e270ad3c03ce70cbd1c5f51f9d4f24  crates/incant_render/src/ui/theme.rs
fee1ebcf26633a9e8905f7efd42fc844a40f0686f137865506ac11f2613b6169  crates/incant_render/src/ui_draw_tests.rs
1bbc216032eaefe9460ae1ebbe64d6b512d0423959e88dbd3734fabf48c2ca66  crates/incant_render/src/ui_tests.rs
db64cadff721e989f7b49be946020978b2102173693f54c5f45e88892c8b5647  crates/incant_render_data/src/lib.rs
cfcb10b0796899c46e7db0aaaafe3b4c34ff779638b5673c154727503271363a  crates/incant_render_data/src/ui.rs
12c57a930d79bbdaf9728f8274c5e4b9df5b30d7c2d0392beb25064461ac404c  crates/incant_render_game/src/ui.rs
429ba57c69f79a6784c4297dfc1fe76c2c4259a68677b47a55c915ac3389f1be  crates/incant_text/src/atlas.rs
33fbcadd772fae665534d63f0aaf37217163660a4df06648b0e700eae25c8054  crates/incant_text/src/halo.rs (new) [this pass]
37c87a4bfd0787db2279073c85fa4994c4047afd182ab742c7e5300a6f04a886  crates/incant_text/src/lib.rs
c644bf379f083794d2611d89c8fedfad85e7e777577e0c83934cccefc19ac108  crates/incant_text/tests/atlas.rs
329d2d63b1917e3cece9ff92fb9949472ae007d10ab116d09a2e8b81a4df686f  crates/incant_ui/src/fonts.rs
4131d10a3fffb17de27de680073b010213dbfb90ccacce05e9cb7e92edfa5649  crates/incant_ui/src/lib.rs
21e420ac3722133a4eaa60316752c3b13ced972b763ed84a124e852ccda01fba  crates/incant_ui/src/paint.rs
a9a94f9e9e832fb94720f63da85616043428ed1aa3b0c126842c7e2944795401  crates/incant_ui/tests/fonts.rs
833bd2e912dffe7e8e5c970c26bcec97321518f2c17a519f8c5380e946c6bfd4  docs/spikes/native-ui-draw.md [this pass]
974101cc51be4af23ffeba5f5485ff91cd933fce1d6cb0836ba95f18e035d8f3  tools/build_variant.py (new)
791a46e80f7aaf04a57500da4e1a32c196f5323d2835775f65adee391064e2e7  tools/capture.py (new) [this pass]
bc400af199d76289db71f5b1579e5f2fa97d4aea3450e7d7aa6e1e48794458d8  tools/frame_profile.py (new)
f8658880572979fec9ffead532ba9c613fd7fee18da1ac18d0abeedefceca4a4  tools/lease.py (new) [this pass]
0f98ab942e5a1ecda2c50c958d8cf35480d9d7e974ae5bf2567c7e34a593b77c  tools/placement.py (new)
36bde5d14f52debea567763db8cdfd804295a89e6ed85d678b1ff6b4f59a76b5  tools/plans.json (new) [this pass]
b903c2e2ae4bffdfdfb5011ccb31940cfedb16f337242d0b5eca174ead494a0e  tools/regression.json (new) [this pass]
db285d247f50db932c89ab01119b552072591be1a0107a809aa2755592251088  tools/regression.py (new) [this pass]
e4dbd020d7d518b74720cf6b50e2ee40767c7dcc139021debaf376936e407acf  tools/result_hud.rs (new)
dba1f4fa352495a4743c71c306fa9fe0011ea362a26b1b878edbde4c43ab1d99  tools/routes.py (new)
15d0923c2d626ff04c3254e7657a413406112f29e86ae384217b3a19053d873e  tools/test_lease.py (new) [this pass]
9fa7fd1d012688e4dec8d0e3983235683e0d04f0acd90d20a25caa4b14d78826  tools/text_width.py (new)
4a54de7dfd55c7afaf4148ce20a243882952f253c39af53aab8872e4c9029ae1  samples/core/PRESENTATION.md
a6019be943514c7163706be399e6675d7aa3b8c7e8933d7107469459fe9fe99a  samples/core/ui/hud.json
0e21d0c1c648b9fa297797e796abc507a8cf35494ca898bee96921e20f1bda91  samples/core/ui/strings.json
054bc3eb51d4fdf86364a0427310b154a264b5232df609e8d1ef53bcfd9b8142  samples/planar/PRESENTATION.md
ca7f7e4a900fc33b8f31c4b500dd378a5682786713455ee436a7e44023dea4c8  samples/planar/hud.py
```

`regression.json` still pins `ui_draw_tests.rs` (above) and
`crates/incant_render_gpu_host/tests/ui.rs` (`7ebe885a…`, unchanged).

### Suggested CPU sequence for root's supervised grant

Use `./tools/cargo … --locked -j2 --profile ci --offline` with
`CARGO_TARGET_DIR=/tmp/incant-agent-font-target` in a constructed environment.

1. Regenerate the SC subset with fonttools 4.60.1 per `provenance.json`: twice,
   plus the committed-hash control. Then update the provenance and confirm with
   `fontcheck` that every authored glyph is covered in all four locales.
2. Run `cargo test` for `incant_text`, `incant_ui`, `incant_render_data` and
   `incant_render_game`, and `cargo test -p incant_render --lib`. These are CPU
   tests; GPU cases stay ignored. They include the halo, atlas, paint, plan,
   refusal and reference-mutation tests.
3. Run `cargo clippy --all-targets -- -D warnings` for those packages, plus a
   workspace `cargo check --all-targets` for downstream users of `GlyphKey`,
   `UiPaint::prepare` and `UiWidget`.
4. Run `regression.py build`, which uses `cargo test --no-run` and builds only
   the test binaries.
5. Run the sample's public `verify_bindings.py prepare/ship` routes and the
   layout-only adapter runs, then stop before any GPU work, which needs its own
   grant.

Nothing in this sequence has run.
