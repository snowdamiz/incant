# Core Sample gameplay presentation bindings

The Core Sample controller accepts optional `hud` and `audio` role maps in its
normal authored Script props. They contain canonical stable IDs. They do not
create entities, import media, change project documents, or own layout. Claude's
level builder owns the actual widgets and art; the runtime controller updates
committed native declarations through the same typed command buffer as other
scripts.

`hud` names a string table and seven distinct widgets: `health`, `collected`,
`objective`, `won`, `lost`, `result`, and `inactive`. The two result labels move
between the authored result and inactive containers when the phase changes. The
inactive container's authored clipping controls its visibility. The controller
updates text only when health, collection progress, or the locale changes; a
fresh VM refreshes its declaration cache from persisted gameplay state. It uses
`hud.health` with `health`, `hud.samples` with `collected` and `total`, and the
`hud.objective.collect`, `hud.objective.exit`, `hud.won`, and `hud.lost` keys.
Missing keys fail explicitly. Locale fallback and pseudo-localization use the
existing localization runtime.

`audio` optionally names distinct sources for `music`, `pickup`, `damage`,
`won`, and `lost`. Music starts once for a new run. Accepted pickups and damage
restart their respective one-shots. A terminal transition stops music and starts
its result cue once. Later ticks in that terminal phase do not replay the cue.
Source clips, spatialization, gains, looping, and streaming remain authored
properties. Explicit restarts use the checked native transport epoch; there are
no script-owned mixer handles or repeated per-frame start requests.

The original seven numerical gameplay scenarios pass with these optional maps
absent. Strict TypeScript and the combined real script HUD/audio tests pass.
`samples/core/verify_presentation.py` authors a numerical UI, imports synthetic
WAVs through the public importer, checks Undo/Redo and journal reopening, and
exports an ordinary player. Its observer wrapper reads actual committed HUD
snapshots and requests a French locale change; it does not replace the controller
or any host operation. It checks state-driven text, result parenting, PCM and
repeatability after removing its private authoring inputs and source maps.
The shipping proof passes three fresh win/loss/repeat processes, including actual
next-tick health/progress text, locale switching, result parenting, exact repeated
PCM and terminal silence. A helper initially expected a full-health win despite
the existing gameplay taking one hit; the corrected expectation passed against
the same frozen player without rebuilding. The gameplay source did not change.
Independent final source and retained execution review is accepted. No
finished-level, pixel, phone, performance or phase gate is claimed. [Retained evidence](evidence/core-presentation-bindings.json).

An initial development-route attempt exposed an existing limitation: the
headless `native-play` session has no screen-space UI viewport or recorded UI
routing yet, so it refuses UI scenes. The ordinary game player hosts the UI and
is used by this proof. Physics save/load support and full sample persistence
remain separate open work; the presentation cache does not claim to close them.
