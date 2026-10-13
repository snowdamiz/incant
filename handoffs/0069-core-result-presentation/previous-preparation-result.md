# 0069 Core result presentation — follow-up preparation (delta)

## Status

**Prepared for correctness review. Not executed, and no visual acceptance claimed.**

I didn't use Cargo, a build, the target, the GPU or any capture. No grant exists,
and I didn't wait for one. No memory or external notes were written.

Everything below is uncommitted, so root can review the working tree against
`f5eba1d7`. Astra's prompt-fixture commit is preserved, and it is HEAD.

## Model and effort

Claude Opus 5.5 (`claude-opus-5-5`) with thinking effort **max**, as this session's
context reports. Root's ACP record is the verification.

## Brief and review read

- Brief SHA-256 `6ec6bb6f…`. Root revised it mid-session, from `b9d7254f…`, and I
  re-read it.
- `root-helper-review.json` SHA-256 `456302ca…`, root-owned.
- Both are left unedited and uncommitted. Items 1–6 and all five findings are
  addressed below.

## Design decision: a label halo in the theme, and a fixed top bar

The 0065 failure came from a missing capability, not from placement. Labels drew
light text with a 1 px drop shadow and had no surface. My first proposal only moved
the failure from the title to the stats. The fix belongs in the theme.

**Label halo.** Every label glyph first draws an opaque `#0A0C10` halo: its own
coverage dilated by 1/16 em.

- **Radius.** At least 1 px, at most 2.5 px: 1 px at 12–17 px, 1.375 px at 22 px,
  2 px at 32 px.
- **Look.** It keeps the glyph's antialiased edge, so it reads as a crisp outline,
  not a blur or a glow. Over dark scenes it nearly disappears.
- **Contrast.** Text measures 16.8:1 against the halo over any backdrop.
- **Why opaque.** Translucent halos overlap between letters, double-darken to 98%,
  and look blotchy; an opaque one doesn't.
- **What it replaces.** The 1 px shadow, one quad per label glyph, so quad bounds
  are unchanged.
- **Sampling.** The coverage is the largest of 25 bilinear taps, each clamped to
  the glyph's padded texels: the point, 16 directions at r, and the odd 8 at r/2.
  Minification averages the same N×N grid that glyphs use.
- **Clip.** The text clip grown by r, never past the visible border box, and empty
  when the text clip is empty. Parked or fully clipped labels draw nothing, and a
  label with r of padding keeps its whole halo.
- **What stays the same.** No new authored field, schema, runtime or mutation path.
  The shader's IO, varyings and resources are identical, and every substring the
  shader-cook tests pin is present once.
- **Scripts.** RTL, CJK and Arabic need nothing special, because the halo is drawn
  per glyph.

**Core HUD** (`ui/hud.json`, `ui/strings.json`):

- **Fixed top bar.** It never moves, and the objective clears on a win (controller).
- **Result block.** Title (32/40) and prompt (17/22) are centred in a box 40% of the
  content height under the top bar. That puts them near the upper third,
  above the surveyor.
- **Copy.** The prompt reads "Press R or Start to play again", «اضغط R أو Start
  لإعادة اللعب» and "按 R 或 Start 再玩一次". The legend is "WASD move · Space jump".
- **Padding.** Labels have 2/1/2/1 px of padding for their halos. The root padding
  and gaps shrink to match, so text insets stay 22/16 px as in 0065.
- **Chinese wording.** It is a real choice, not a glyph dodge. The pinned upstream
  `NotoSansSC[wght].ttf` (`a3041811…`), its OFL (`1c05c68c…`) and the fonttools
  wheel (`a884aef0…`) all match provenance, read-only. That upstream maps all 33
  Han characters, including 按 and 或. Regenerating the 44-glyph subset waits for
  execution.

`samples/core/PRESENTATION.md` and `docs/spikes/native-ui-draw.md` describe this
and claim no rendered result.

## Root's five findings

| Finding | Change | Control (`tools/test_lease.py`) |
|---|---|---|
| 1. Interrupts released the lock without killing the child | `supervise` owns each child in a new session. Termination signals are blocked during spawn and cleanup, and the child starts with an empty mask via `preexec_fn`. SIGTERM/SIGHUP raise SystemExit(128+n) while a child runs. A `finally` kills (TERM, then KILL) and waits for the group on every outcome, then re-raises. `run` unlocks only after that and logs the exit in a `finally` | Normal and nonzero exit (each leaves a grandchild), timeout, a group ignoring SIGTERM, raised KeyboardInterrupt/SystemExit, and real SIGINT/SIGTERM/SIGHUP. Every pid and group is gone, and the lock is free afterwards |
| 2. Stripped porcelain lost the first path | `status_entries` parses unstripped `--porcelain=v1 -z`, with explicit renames and deletions | A throwaway repo whose first row is ` M`, plus a rename, a deletion, an untracked file and a path with a space. This worktree's first row now reads `crates/incant_render/src/ui/geometry.rs` |
| 3. Review reuse proved nothing | Reuse is removed. All 22 planned frames render, 4 review and 18 final, within 24 | Case list recounted: 22 |
| 4. Build not bound to capture | Freeze generations. A build must match the current generation before and after compiling, with `crates/` exactly as frozen. `package.json` records the generation and a hash inventory, unchanged across the startup check. `capture.py` refuses any other generation, adapter source or changed file. A reviewed change after review needs an explicit `freeze --new` | Generations match, refuse on change, and step only explicitly |
| 5. Debit outside the lock | `run(debit=…)` debits atomically (temp file, fsync, rename) under the same lock as launch and cleanup | Over-ceiling, negative, boolean, fractional, duplicate and empty debits refuse before any launch. While another holder has the lock, nothing is debited or started |

Brief items 5 and 6 are also covered:

- Children get a constructed environment. It contains the pinned PATH, including
  the resolved fnm Node 22.23.3 for the TypeScript cook, plus HOME, CARGO_HOME,
  RUSTUP_HOME, PYTHONUTF8 and TMPDIR. Nothing is inherited, and only
  `CARGO_TARGET_DIR` and `INCANT_NATIVE_PROJECT` values are logged.
- `acquire` checks any grant `sources` [{path, sha256}] and `predecessor_release`
  {path, sha256} once.
- The adapter header now says fresh single-frame renders make no claim about
  accumulated history, and that history-sensitive cases need their own route.

## Outstanding correctness review (Astra / root)

1. **Renderer source is unbuilt and its WGSL unparsed.** No validator is installed,
   and Cargo is not allowed yet. rustfmt (pinned 1.99.0 toolchain) did parse all
   touched Rust files and applied formatting to my hunks only. The first granted
   steps should be:
   - `cargo test -p incant_shader_cook --test ui --test geometry`;
   - the CPU tests in `cargo test -p incant_render --lib ui`, including the
     reference self-test.
2. **Halo cost.** 25 texture taps per halo fragment, N² more when minified. No new
   GPU resources. The phone cost is unmeasured and not claimed.
3. **Reference and tests.** The reference `halo_pass` mirrors the shader in all
   three precision models; `glyph_pass` was extracted into a shared `tap`
   unchanged. New coverage:
   - CPU tests: halo layering, radius rule, padding, ancestor clip, a parked label,
     disabled state, non-label roles, and the WGSL direction table.
   - Reference self-test mutations: halo absent, and half radius. Their rejection
     margins (outside > 20, excess > 8) are predictions to confirm on the first
     CPU run.
   - A new GPU test draws a label over white and black.
4. **GPU budget decision for root.** The renderer GPU draw tests make about 18 small
   draws (≤320×180). Please say whether they count against this packet's 24.
5. **Other users.** Labels without padding keep a halo clipped at their content box.
   `samples/planar` docs still describe the 1 px shadow; I flagged them but didn't
   edit them. `docs/spikes/core-result-hud.md` and STATUS.md are root's to update.

## Plan after the grant (unchanged parts in `tools/plans.json`)

1. Acquire.
2. Font subset, twice plus the committed-hash control, then provenance.
3. `freeze`.
4. Renderer and cook tests, as root decides.
5. `scene.py` for four locales, `verify_bindings.py prepare` and `ship`, unit tests,
   `tsc`.
6. Packages and adapters, then 12 zero-frame layout runs.
7. Four review frames. If anything changes, a reviewed `freeze --new` and rebuild.
8. Eighteen final frames.
9. Route, PCM and restart checks; layout checks; pixel identity after R and Start.

For legibility I'll judge each label by its text-core to halo-ring contrast, by
whether the halo is clipped by its box, and by 0065's scene-median contrast as a
comparison.

## Run in this session (local, nothing built)

| Command | Result |
|---|---|
| `test_lease.py` | 12 controls OK in 8.3 s; no surviving synthetic process; scratch removed |
| rustfmt 1.10.0 on `ui/mod.rs` and its submodules | parsed; formatted; clean |
| Static `hud.json` check | styles valid; prompt contract, reparenting and RTL correct |
| Read-only cmap of the pinned upstream font | covers all 33 Han characters needed |
| `source_identity()` on this worktree | complete paths |
| `lease.py status` once, then 7 guarded commands | no grant, lease or target user; each command exits 3; evidence directory never created |

**Not run:**

- any Cargo, engine, renderer test, sample unit test, `scene.py`, verify tool,
  export or GPU work;
- screenshots: there are none.

## Changed paths (uncommitted)

- Renderer: `crates/incant_render/src/ui/{theme.rs,geometry.rs,mod.rs,ui.wgsl}`,
  `crates/incant_render/src/{ui_tests.rs,ui_draw_tests.rs}`,
  `docs/spikes/native-ui-draw.md`.
- Sample: `samples/core/ui/{hud.json,strings.json}`, `samples/core/PRESENTATION.md`.
- Packet tools (`handoffs/0069-core-result-presentation/tools/`):
  - runner: `lease.py` `21aa60b1…`, `capture.py` `da58b901…`,
    `build_variant.py` `974101cc…`;
  - adapter and plan: `result_hud.rs` `e4dbd020…`, `plans.json` `32f67371…`,
    `routes.py`;
  - controls: `test_lease.py` `4155d2d1…`;
  - planning: `frame_profile.py`, `text_width.py`, `placement.py`.
- This `result.md`.

## Limitations

- **No acceptance yet.** All appearance, contrast and readability judgments wait for
  the granted frames. The figures above are theme constants, not measurements.
- **Pseudo-locale at 420×320.** The wrapped title and prompt can reach the
  surveyor.
- **Legend.** It stays keyboard-only for move and jump.
- **No gate.** No device, performance or phase gate is claimed or approved.
