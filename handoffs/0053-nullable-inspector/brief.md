# Focused native Inspector recheck after 0052

Claude Opus 5.5 through ACP, with Max thinking explicitly verified, owns this
native appearance check. Read `build.json` and the completed 0052 result first.
This is a narrow follow-up to its concrete Clip-field defect. The full animation
transport/resize review already passed; do not repeat that entire session.

## Build and authorization

Use only the copied `incant_editor` binary named in `build.json`. Verify its hash,
the build manifest and named fixture before launching. The source is root's
2841c24: nullable type-array normalization in `editor/bridge/native.ts` and its
generated JS are fixed. No layout or styling changed. The actual Animation schema
uses `type: ["integer", "null"]`; it now maps to the existing nullable integer
field, preserving bounds, optionality and authored values. Fifty-two bridge tests,
906 UI tests, strict app/test TypeScript, generated transport and production build
pass. Independent Sol source review found no issue. This is a `ci` profile build,
not a shipping performance measurement.

The native/GPU interval is granted exclusively to 0053. All 0052 apps and helpers
have exited and its grant was released. Use fresh 0053 helper paths; do not reuse
0052's released marker or change its evidence. No provider turn is authorized or
needed. Do not open account/auth dialogs, inspect credentials or alter user apps.
Wait for a short idle interval before taking the front, stop input if the user
switches apps, and restore the prior front app/pointer after each short burst.

## Check

1. Open the same retained v2 project identified in `build.json`. Use the working
   native helper approach from `handoffs/0052-native-animation/tools/native/` and
   its Node PATH when needed. Do not rebuild, mutate or replace the binary.
2. Select the rig and inspect its Animation component. Clip must now show its
   actual numeric value 0 through the existing field presentation, with no
   unsupported-type notice or raw-JSON fallback. Check that adjacent values and
   labels remain intact and readable at about 1440 by 875 and 1000 by 650. The
   OS may clamp the large window height; record the actual size.
3. A brief Play/Pause/Stop check is enough to verify the corrected Inspector
   remains usable alongside runtime state. This does not replace 0052's transport
   evidence. No authoring edit is needed. Keep the same project/journal/behaviour
   files unchanged and verify their final hashes.
4. If a real appearance defect remains, report it precisely. Bridge/data binding
   changes belong to Astra; visual presentation belongs to you. Do not start an
   unrelated redesign or revisit the previously accepted whole-app surfaces.

At most three native window captures, plus your own crops if useful. Claude
alone reviews pixels and redacts the account avatar in committed captures. Keep
raw captures local and ignored. Use no external screenshots or generated art.

## Report and cleanup

Return `result.md` with the verified model/effort, actual build/fixture identities,
scope, result, screenshot paths and limitations. Stop and reap every app/helper
you own and record native/GPU release before finishing. Commit evidence with
`Built-by: claude`; do not publish or merge. Do not claim phase, platform, device,
IME, frame-pacing or shipping-performance gates. The separate missing Console
logs observation is being implemented by Astra/Sol and is outside this recheck.
