# Authored Core runtime bindings

The authored 0060 mesa now binds its existing HUD widgets, localized table and
audio sources to the current native Core controller. `samples/core/bindings.py`
uses one public three-command transaction: checked script replacement, existing
Script props plus role IDs, and the existing input actions plus restart. The
builder uses the same role map. The controller is byte-identical to main181;
Claude's art, layout, media, movement and physics choices are unchanged.

The existing localized health/progress/objective and outcome declarations now
update through native commands. Pickup and damage restart their existing cues;
terminal outcomes stop music and start the result cue once. The first existing
pickup/hurt source is reused; the other authored sources remain unchanged and
silent. KeyR and gamepad Start request a fresh whole-session replacement after
a terminal state. No partial entity reset or physics checkpoint is involved.

Execution and independent final audit are accepted; publisher intake is pending.
[Final review receipt](evidence/core-authored-runtime-bindings-review.json).
[Source and retained execution evidence](evidence/core-authored-runtime-bindings.json)
bind the exact helper, engine inputs, player, package files, process requests,
outputs and PCM. The package was built from main181 with only the production
controller attached. Its private authoring inputs, source maps and build storage
were removed before these runs.

- Six fresh package baselines cover the full 2,156-tick win, loss, alive locked
  exit, idle, extended terminal silence and a shorter complete loss.
- Four complete resets cover win-to-loss, loss-to-win, gamepad restart and held
  movement/jump isolation. Each compares final logs, entities, shaped UI and
  PCM against a fresh session; the complete PCM equals the two fresh recordings
  concatenated exactly. Host chronology stays monotonic while simulation resets.
- Fourteen successful native processes cover the authored routes, actual
  pre-command HUD readback, English/Arabic/Chinese/pseudo-localization and four
  cue-isolation controls. The separate test observer never enters the export.
  Cue subtraction is nonzero at every corresponding gameplay event and exactly
  equal outside bounded cue windows.
- Wrong-role, wrong-controller-source and missing-table controls use real
  public project edits and Undo. The final three-command binding transaction
  passes Undo/Redo and journal reopening. Original content remains unchanged.
- The original seven numerical fixture scenarios still pass separately. They
  are not described as the authored mesa. Sixteen focused Python boundary tests
  and strict TypeScript pass; the matching native host was built successfully.

Earlier helper failures remain retained. Undo changes project revision even
when source/version/history restore exactly; an empty table map can be omitted
by canonical serialization. The first shipping attempt successfully built and
ran five baselines, then refused two full wins because 4,312 host ticks exceed
the existing 3,600-tick CLI bound. Complete win/loss runs in both directions fit
that bound without shortening the full win. A subsequent recording helper
incorrectly stripped a needed gamepad connection after a no-pad loss; the final
helper carries actual physical topology and suppresses only duplicate connects.
Both shipping corrections replayed the exact original package without recooking.

Headless `native-play` still explicitly refuses session replacement because it
does not own that lifecycle. The ordinary packaged player performs all four
resets. No GPU, native-window, appearance, phone or performance gate is claimed.
The visible restart hint remains a Claude presentation follow-up; no inert prompt
was added. Physics save/load remains separate. The historical 0060 offscreen
helper source-hash gap remains unresolved; this new CPU proof does not rewrite
that earlier provenance claim.
