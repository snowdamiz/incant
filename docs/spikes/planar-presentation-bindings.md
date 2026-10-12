# Planar optional presentation bindings

The existing Planar controller accepts optional authored flipbook, HUD and result
roles. The controller uses public typed native commands; it does not build UI,
choose appearance, change authored content or replace the original movement and
sensor logic. Omitted roles keep the original seven-scenario contract.

Bound player and boat flipbooks are paused state indicators. The player uses
frame 0 during play, 2 after winning and 1 after losing. The boat changes from 0
to 1 when all pickups are held. HUD text uses the current localization settings,
with `hud.collecting`, `hud.ready`, `hud.won` and `hud.lost`; result nodes move
between explicitly authored active/inactive parents. Commands run initially and
on relevant changes, not on every unchanged tick. The first callback accepts the
initial HUD before the first nonzero movement integration. Reads within that
callback remain pre-command snapshots.

The numerical presentation fixture is authored through public project commands,
with actual Undo/Redo and journal reopen checks. Its wrapper delegates unchanged
gameplay and reads committed flipbook/HUD snapshots; it requests French at tick
50. Seven real native runs check keyboard/gamepad victory, hazard loss, wall
collision, locked exit, normalized diagonal motion and repeatability. Their
original gameplay records, final player observations and PCM hashes exactly match
all seven runs without bindings. Four actual malformed optional-role launches
refuse with precise diagnostics and no PCM, while authored files remain unchanged.
Four mutations of actual accepted reports are caught by the oracle.

The initial observer incorrectly treated an absent non-text `text` property as a
string because it tested against null. Native execution completed, but the helper
assertion failed. The original run is retained; changing only that observation
predicate to `typeof text === 'string'` made the full presentation proof pass.
The gameplay source is unchanged from independent source acceptance.

Two independent fresh shipping exports pass fourteen copied-player processes:
seven default and seven with presentation bindings. Each set executes 1,860 ticks
and 1,488,000 stereo frames. Every run has exact development logs, observations
and PCM hashes. The presentation package also reports the final shaped localized
status, empty prompt, seven canonical widgets, nonzero glyphs and the explicit
640×480 logical viewport. The copied player runs after deletion of its own
project/cache, private build storage and external script maps. The original
native evidence remains separately retained outside the isolated player path.

The original default export compiled but its audit rejected the newly split
player library/executable pair before publication. The separately reviewed
f28b5441 repair checks required flags on both units and final fat LTO on the
executable; both real shipping exports now pass that four-unit audit. The first
presentation build was interrupted, with unknown exit and no published package.
Its successful fresh retry retained the same 1,759 source/build input identities.
Neither failed nor interrupted attempt is counted as a successful export.

Strict TypeScript and convention/diff checks pass. The five sample/helper inputs
are frozen in source002. Source, local execution and final shipping reviews are independently accepted. Actual main170 differs from the tested f28b5441 base
only in documentation and retained exporter evidence; compiled inputs are exact.
[Retained manifest and raw/compressed artifact identities](evidence/planar-presentation-bindings.json).

The one-pixel atlas, repeated test frames and neutral widgets are numerical test
data. Claude owns Pearl Cove art, actual layout and appearance acceptance, in a
separate worktree. No native-window, GPU, phone or performance evidence is claimed.
PCM is silent, so parity establishes deterministic frame/sample accounting here,
not audible cue behavior. Physics checkpoint continuation remains unsupported.
Whole-session restart is still unmet: the prompt is cleared, and no inactive
restart key is advertised. A valid restart must restore world, solver, script and
host ownership together; this change does not imitate it by resetting a few
entities. Existing follow-camera behavior and simultaneous hazard/exit priority
remain outside these seven scenarios.

The manifest retains the independent source and local receipts as compressed
artifacts, with their original SHA-256 identities. It also records both actual
player binaries, complete package file hashes, all fourteen shipping command
receipts and the original negative/interrupted controls. No large executable is
embedded in Git; its retained local path, byte count and hash are recorded.

[Independent final receipt](evidence/planar-presentation-bindings/final-review.json)
rehashed the source/build freeze, 233 raw/retained artifact pairs, all three
binaries and both package inventories, then compared all fourteen process
outputs with their native evidence. It approves this numerical scope only.
