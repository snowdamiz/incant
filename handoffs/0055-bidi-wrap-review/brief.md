# Recheck mixed-direction text wrapping

Claude Opus 5.5 via ACP, explicitly verified Max thinking, owns every rendered
review and screenshot. This packet follows your accepted 0054 canonical native
text draw result. Root has repaired the nonvisual mixed-direction wrapping defect
that you reported. Read AGENTS.md, CLAUDE.md, your retained 0054 result, and
`docs/spikes/text-bidi-wrap.md` with its source-review and test evidence.

## Scope

Review the corrected actual text layout through the same canonical native draw
route. The pinned cosmic-text 0.14.2 now uses a small reviewed vendor correction;
there is no shaper upgrade, hand-adjusted glyph placement, widget layout, or new
render mode. The source delta is `third_party/cosmic-text/INCANT_PATCH.diff`.
Do not edit that numerical foundation in this packet. If a new numerical defect
appears, return its exact source string/fonts/metrics and root will repair it.
You may fix an actual draw defect within your existing renderer scope, with tests.
Do not restyle the editor or broaden this into game UI implementation.

Check `Wrapped ffi سلام 中文 ש and spaces` at 28 px, line height 36, width
300 and 180. Also include an RTL-base mixed paragraph and a narrow long-token
case, e.g. `سلام ABCDEFGHIJKLMNOPQRSTUVWXYZ שלום xyz` at width 32 or 60.
Use exactly the explicitly packaged Latin, Arabic and CJK chain from 0054;
uncovered Hebrew is intentionally an observable .notdef glyph. Confirm that the
interior space remains, the visible words wrap, and no non-whitespace cluster is
lost. Word mode deliberately permits a word larger than the container;
WordOrGlyph keeps indivisible glyphs intact even when a glyph itself is wider.
Keep spaces/ligatures/Arabic joins/CJK fallback and normal transformed text intact.

## Build, fixture and checks

This checkout combines accepted 0054 drawing with current main's fixed player
observation routing and root's reviewed bidi repair. Build from this exact source,
record actual source tree, binary/profile, fixture, font and capture identities.
Do not use the old 0054 executable or mutate its evidence/project/package.

Author a new disposable fixture through public imports and incant_cmd, adapting
`handoffs/0054-native-text-draw/tools/fixture.py` if useful. Use the ordinary
native package/window or editor Play path. A fresh `ci`-profile player is adequate
for this focused correctness review; label its profile and make no shipping or
performance claim. If using normal export, use a new private output and recook
with this exact Cargo lock. No provider/account action is needed or authorized.
Run the focused shaping regressions and renderer text numeric checks; retain
existing failures, do not substitute screenshots for tests. The previously failing
native-window routing test is repaired in this main base and should now pass.

Take a small useful set of actual app/window captures at readable default and
resized bounds. Inspect pixels yourself. Root never approves pixels. Preserve
0054 originals and keep raw captures locally if any account/avatar appears;
redact only that area from committed artifacts. No system font substitution.

## Exclusive native/GPU and build grant

0054's native/GPU/target grant is released, all its processes ended, and root
reaped ACP handle93339 with exit0. This packet owns the exclusive native/GPU
lease and `CARGO_TARGET_DIR=/tmp/incant-agent-native-target` until cleanup.
Other lanes remain CPU-only. Use `./tools/cargo`, `--locked`, bounded `-j2`.
Do not use or delete any other target/cache. Do not reopen0054's released tools;
copy/adapt helpers inside0055 with fresh paths and a new lease marker.
Wait for a short idle interval before focusing the app. Use short review bursts,
restore the previous front app and pointer, and yield immediately to user input.
Avoid the earlier launch PID error: record the actual child/player PID and use
`exec` in wrappers. Any helper polling must not match its own command line.

## Return

Commit only your packet tools/evidence/result and any necessary reviewed draw
correction, each with `Built-by: claude`. Return `result.md` with explicit
Opus5.5/Max verification, reproduction strings/metrics, exact results, image links,
source/binary identities and remaining limitations. Say clearly which native
entry point was exercised. No phone, full Unicode, Game UI or phase gate claim.
Stop and reap every owned player, renderer, helper and build; record the release
of native/GPU and the target cache before returning. Preserve imperfect outcomes.
