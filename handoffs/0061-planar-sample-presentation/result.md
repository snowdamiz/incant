# 0061 result: Pearl Cove with a live, localized engine HUD

## Status (priority revision, 2026-10-12)

- **HUD revision: complete for offscreen scope.** Pearl Cove now has an actual
  engine HUD: an imported font, UI nodes, a localized string table and the
  controller's presentation bindings. It was verified by:
  - seven native-play scenarios;
  - offscreen GPU renders of live states from the embedded program, in five
    locale variants and three window shapes;
  - the ordinary player's CPU UI host.
- **Native-window review: still pending.**
  `~/.codex/incant-evidence/0060/released` was absent at 21:49Z. I opened no
  window and captured nothing. The pending item is below.
- **Restart: open.** The prompt is deliberately blank.
- No feature, device or phase gate is approved.

Model: **Claude Opus 5.5 (`claude-opus-5-5`)**. Thinking effort: **max**. The
root holds the ACP-side verification record.

Brief applied: SHA-256
`95797d9e7f20481487dc1c3ec9382f86452a4f08cc8e5856a2d992ea4f67f274`, including
the "Priority revision — actual HUD and current runtime". The brief is unchanged
by me.

The first pass is preserved verbatim in `result-first-pass.md`, with its
evidence in `evidence/` and `screenshots/`. This revision's outputs are in
`evidence/hud/` and `screenshots/hud/`.

## Revision feedback applied

| Revision item | Done |
|---|---|
| Build on the original art and level with a restrained, polished HUD | One centred status line. The top two cliff rows became a quiet moss variant as its ground. Art and level are otherwise unchanged. |
| Live progress and clear terminal states | Live: Pearls k of 3, then the ready line. Outcome sentence, plus the bound pose (cheering or stung) and boat-lantern frames. |
| No static labels as live state; no baked text | Both labels are authored blank. All text comes from `setUiText` with engine shaping and Inter. |
| Imported fonts, UI documents, atomic public commands | Inter (SIL OFL 1.1) is copied with its license and imported with `incant_headless import`. Transaction 4 adds the string table, locale, UI nodes and controller bindings atomically. |
| Optional props, distinct canonical ULIDs | `flipbooks {player, boat}` and `hud {table, status, prompt}`. `results` is deliberately unbound (see below). |
| Localized StringTable entries | en, es, fr, de with ICU plurals. Pseudo-localization verified. |
| Blank prompt; report restart as open | Prompt is blank in every capture; restart is listed as open. |
| Fresh headless; fresh lease; at most two jobs | Headless rebuilt from `44060bc3`. New lease in `~/.codex/incant-evidence/0061-hud`; the old guard still refuses. |
| Exercise keyboard/gamepad win, loss, locked exit with the HUD; offscreen renders | All exercised, with live HUD text asserted at every capture (below). |
| Do not edit `gameplay.ts`, `verify.py`, `export_verify.py`, `tsconfig.json`, `presentation.py` | Untouched. `presentation.py` does not exist in this branch. |

**Why `results` is unbound.** That binding only reparents the result labels;
the controller never writes their text. Any authored headline would stay
English in es, fr and de and in pseudo mode, and it would repeat the localized
status sentence. The status line carries the outcome. To add a terminal
headline, have the controller localize the result labels on each locale
refresh, as `samples/core/gameplay.ts` does, then bind `results`.

## Changed paths

| Path | Change |
|---|---|
| `samples/planar/hud.py` | New. UI document (root, status, blank prompt) and the four-locale string table. |
| `samples/planar/scene.py` | Font copy and import, and transaction 4 (HUD). Undo/Redo across all four transactions. |
| `samples/planar/playtest.py` | Native-play now passes `--ui-viewport 640x360`. |
| `samples/planar/art/terrain.py`, `art/cove.py` | Quiet moss variant for the top two rows. |
| `samples/planar/PRESENTATION.md`, `README.md` | HUD design, IDs, strings, results and restart decisions, final budget. |
| `tools/tests/test_planar_presentation.py` | HUD test added. HUD IDs are covered by the canonical-ID test, which would have caught the invalid `UI` kind code. |
| `crates/incant_game_player/examples/hud_frames.rs` | New review example. It runs the package's build-embedded program with the GPU UI consumer offscreen and writes frames plus engine HUD text. It is feature-gated like `macos_text_retirement.rs`, with no manifest or engine change. |
| `handoffs/0061-…/tools/` | `lease.py`, `hud_review.py`, `collect_hud.py` added; `mutate.py` extended. `guard.py`, `review.py` and `collect.py` stay as first-pass history. |
| `handoffs/0061-…/result-first-pass.md` | The first pass's `result.md`, renamed unchanged. |

## Commands and results

Lease: `~/.codex/incant-evidence/0061-hud/lease.json`, started
2026-10-12T21:29:49Z with a 12-hour expiry, released **21:50:00Z**. After
release `check`, `cargo` and `run` refused (exit 3). The ledger holds 89
entries (`evidence/hud/ledger.jsonl`). Every cargo call used `./tools/cargo …
--locked --profile ci -j 2`, Rust 1.99.0 and the assigned target.

| Step | Result |
|---|---|
| `lease.py cargo build -p incant_headless --bin incant_headless` | Exit 0 in 1m48s. `incant_headless` `614fe208…1707e7a0`, from HEAD `44060bc3`. |
| `lease.py cargo build -p incant_game_player --example hud_frames --features render-gpu` | Compile check exit 0, 3m56s. |
| `rustfmt --edition 2024 --check` on the example | Initial run found formatting diffs; formatted; final check exit 0. |
| `lease.py cargo clippy … --example hud_frames --features render-gpu -- -D warnings`, and without features | Both exit 0. |
| `python3 -B handoffs/0061-…/tools/hud_review.py ~/.codex/incant-evidence/0061-hud/review-2` | Exit 0. Details below. |
| `python3 -B -m unittest tools/tests/test_planar_presentation.py` | 5 tests OK, also on Python 3.13. Syntax parses for 3.9. |
| `python3 -B handoffs/0061-…/tools/mutate.py` | 10 of 10 detected. New HUD mutations: static `results` bound, a translation missing, non-canonical HUD ID, static text in a live label, unknown message argument. Unmutated suite passes; files restored byte-exact. |
| `python3 -B -m unittest discover -s tools/tests` | 253 tests OK, 4 skipped. |

Inside `review-2`, every child was lease-checked:

- **`scene.py --play`.** History: two imports (atlas, Inter), then four editor
  transactions with 3, 2, 13 and 6 commands. Undo×4, redo×4, save, journal
  reopen and `validate` pass. Seven scenarios run with the HUD bound and
  `--ui-viewport`:

  | Scenario | Ticks | Result |
  |---|---|---|
  | keyboard, arrows, left stick and d-pad wins | 781 each | identical logs, observations and PCM |
  | urchin loss | 201 | lost |
  | locked boat | 181 | no win |
  | shore then cliff | 221 | stopped |

  Play left the authored files unchanged.
- **Locale variants.** es, fr, de and pseudo are scratch copies changed by one
  public `set_locale` transaction each. For each variant:
  - `hud_frames` was built with `INCANT_NATIVE_PROJECT=<variant>`;
  - cooked data was packaged by `tools/native_package.data_package` (663,958 B);
  - real replays were rendered on Metal, Apple M5 Pro.
- **Assertions at every capture (28 in total).** The engine UI owner's status
  text equalled the expected text. The expectation is computed independently
  from the message patterns and the engine's pseudo mapping. The prompt was
  blank. Beachcomber and boat flipbook frames matched the state table. There
  were 0 missing glyphs.
- **Gamepad.** The gamepad win's final frame is byte-identical to the keyboard
  win's (RGBA SHA-256).
- **Determinism.** All 28 captures are bit-identical to the earlier full run
  `review-1`. That run was an independent build with different
  importer-assigned asset IDs and a pre-format example binary.
- **Ordinary player, CPU UI host.** The ci project player (`6056511b…`,
  features `project,native-window`) ran with `--package … --ui-viewport 640x360`.
  Its `render_input.ui.sources` status was:
  - keyboard and gamepad win: "Sailing home with 3 pearls";
  - loss: "Stung by a sea urchin";
  - locked boat: "Pearls 0 of 3".

Two development runs failed and were fixed; both are in the ledger. The first
HUD authoring used kind code `UI` in the IDs; Crockford base32 excludes I and
U, so validation refused it. The first-pass guide had published the same
invalid IDs; it is now corrected. The other failure was the initial rustfmt
check.

## Screenshots (`screenshots/hud/`)

All are actual Incant output: the composed game, NativeRenderHost and
GpuRenderConsumer with its UI layer, offscreen on Metal (Apple M5 Pro), running
the package's embedded program on real replays.

| File | State | SHA-256 |
|---|---|---|
| `01-en-start-1280x720.png` | tick 2, "Pearls 0 of 3" | `d725aae2…cb6adb` |
| `02-en-one-pearl-1280x720.png` | tick 150, first pearl gone | `16c07b1a…000ad0` |
| `03-en-two-pearls-1280x720.png` | tick 450 | `e9733e97…e92284` |
| `04-en-ready-1280x720.png` | tick 560, ready line, lantern lit | `95b04e58…679bf4` |
| `05-en-won-1280x720.png` | tick 781, cheering on the pier | `21c1400e…dfeec6` |
| `06-en-lost-1280x720.png` | tick 201, stung | `fcafdb25…329a7a` |
| `07-en-locked-boat-1280x720.png` | tick 181, at the boat with no pearls | `130faf3f…5e9bac` |
| `08`, `09-en-window-4x3-…-1280x960.png` | 640×480 pt window shape; status in the letterbox bar | `7c74bdbe…0422c8`, `e21a8b1a…fafc84` |
| `10-en-phone-landscape-won-2532x1170.png` | iPhone 13 landscape pixel size; scene 6× | `5d87e3a5…f6e9d6` |
| `11-es-won`, `12-fr-ready`, `13-de-lost`, `14-pseudo-ready` | localized terminal and ready lines | `5fdabe2f…341b36`, `0703ea00…189275`, `55376eca…cd93b9`, `b762be36…1604a8` |
| `15-status-line-all-locales-and-states.png` | top 64 rows of 20 real captures (5 locales × 4 states) | `7525197e…4f35a9` |
| `16-iteration-18pt-line-over-cliff-faces.png`, `17-iteration-before-quiet-moss.png` | rejected iterations | `33964b2b…4a6851`, `45cdaee0…73ff8c` |

## Rendered critique

Fixed during the revision:
- An 18 pt status at a 14 pt inset dipped onto the cliff faces at both ends
  for long strings (screenshot 16). Now it is 16 pt on a 20 pt line at a 6 pt
  inset, inside the 28 pt band above the faces at 16:9.
- Light moss tufts behind thin strokes added noise (screenshot 17). The top two
  rows now use quiet moss.

Accepted:
- **Landscape phones.** The line straddles the 15 pt letterbox bar and the
  moss. Both are dark, so it stays legible. UI layout cannot see the camera's
  letterbox rect, and no single inset fits both full-bleed 16:9 and phone bars.
- **Font.** The whole Inter face, 310 KB cooked, is the largest asset; the
  engine has no subsetting step.

## Budget (desktop, not reference-phone)

- One 126 × 244 RGBA8 sRGB atlas page with 8 mips and 93 frames (78 tile, 15
  sprite), 164,020 B KTX2 per color role.
- Inter cooked to 310,300 B.
- 455 tile cells, 12 sprite entities, 79 flipbook frames and 3 UI nodes.
- 10 to 28 glyph quads and 2.6 to 6.0 KB of UI GPU data per frame.
- `world.scene` 15,854 B; 663,958 B of package data in total.

## Leases, processes and cleanup

- **Lease.** Released as above. The first-pass lease stays released, and its
  guard still refuses.
- **Processes.** No owned process remained. Every child ran to completion, and
  `pgrep` showed the target idle before release.
- **Target.** Its `ci/incant_headless`, examples and player were replaced by
  this branch's builds. The earlier headless binary was `e60b5132…0f909bba`
  (`evidence/hud/headless-before.txt`). Nothing else in the target was
  deleted.
- **Scratch.** Iteration builds under `~/.codex/incant-evidence/0061-hud`
  were pruned. `review-1` (determinism comparison) and `review-2` (final,
  including its packages) remain.
- **No capture lease.** No window was opened and nothing was captured.
- **`node_modules`.** The worktree-root `node_modules` directory was created
  by the integration step at 17:11 local, not by me. I used it read-only and
  left it in place.
- **`__pycache__`.** Directories created by my test runs were removed.

## Pending native-review item

When `~/.codex/incant-evidence/0060/released` exists:

1. Take a fresh lease and capture lease.
2. Check the session is unlocked and has had at least 20 s of HID idle.
3. Build `--bin incant_game_player --features project,native-window` with
   `INCANT_NATIVE_PROJECT` set to the authored project; `hud_review.py`'s
   `player_cpu` step does this already.
4. Launch `incant_game_player --package data --window` as a supervised child.
5. Drive the `playtest.WIN` route in short focused bursts. Take window-only
   captures at start, one pearl, ready, won and loss, then at a 4:3 size.
6. Close through the AX close button, restore focus, and reap everything.

Acceptance means the window's HUD and scene match screenshots 01–09 at equal
scale, allowing for Display P3 capture tagging and the window's own UI
viewport.

## Remaining gates and limitations

- Native-window acceptance, above, is pending.
- **Restart** is not implemented: the prompt stays blank and no control is
  advertised. It needs a `restart` action (KeyR, Enter, gamepad Start and
  South), accepted only when won or lost, plus a session reset that rebuilds
  the world from the cooked scene, clears `context.state`, respawns pearls and
  resets the flipbooks.
- The `results` terminal headline needs controller-side localization before it
  can be bound.
- Everything here is local macOS offscreen evidence. There is no
  reference-device, performance or phone claim. The review builds are
  ci-profile, not the audited shipping export, which is Astra's.
- The `hud_frames` example is a review helper, not a shipped surface; Wasm HUD
  parity is the engine's open item.
- There is no audio. Physics save/load is not claimed.
