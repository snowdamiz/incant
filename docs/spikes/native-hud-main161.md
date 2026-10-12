# Native HUD controls on main161

The reviewed native/TypeScript HUD controls compose with current main's complete
root-motion path. All 20 producer source/test/tool inputs are byte exact. The
current union passes 444 affected CPU cases and 10 player cases, all-target
Clippy, compiler-disabled/static-module and wasm32 checks, strict SDK/generated
contracts, dependency guards and formatting. No renderer styling changes are
included.

A fresh ordinary package passes three 12-tick runs after private authoring/cache/
build input and debug-map removal. Canonical shaped UI/world text, declared
values, focus, native motion and audible PCM are checked. A six-tick save and
fresh six-tick continuation reproduce the exact final save and concatenated PCM.
Three exact role/trap/quota refusals leave empty stdout and no PCM/save output.
Public command-bus authoring, Undo/Redo and journal reopening are exercised.

The [retained source, commands, outputs and package receipts](evidence/native-hud-main161.json)
are accepted by the [independent final integration review](evidence/native-hud-main161-review.json). Exact-head fast CI remains required for publication. Main161 adds only root-motion evidence
to the tested union; all frozen implementation/build inputs remain unchanged.
Wasm HUD parity, native-window/IME behavior, GPU appearance, physical audio,
reference-device performance and phase gates are separate. PCM and save files
are not a multi-file atomic transaction.
