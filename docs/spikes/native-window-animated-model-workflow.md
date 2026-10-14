# Native animated-model editor workflow

The actual Mac editor exercised compatible clip-content refresh through its existing
import, source watcher, Undo/Redo, and native Play handlers on 2026-10-14. The same
world, script module state, and complete animation playback snapshot survived all
six stages. A successfully imported duration change committed to authoring, then
explicitly failed live admission while the prior complete catalog kept presenting.
Restoring compatible source recovered through the watcher without restarting Play.

This extends the [compatible animation capability proof](animated-model-refresh.md)
with an actual app-handler workflow. It does not claim GUI import/history clicks,
captured animated pixels, appearance acceptance, performance, or a phase gate.
The earlier numerical Metal deformation/refusal pixels remain separate evidence.

The opt-in `--animated-model-probe` and animation observer require `play-probe`.
The observer copies native state after checking the Play generation; it adds no
normal editor mutation API. Fixture authoring uses `incant_cmd` through the real
command, import, watcher, and history handlers. A persistent module counter detects
a VM reset; the animation clock is also checked against the authored start and
native tick, then checked again after one normal Step.

| Actual Window001 stage | Authoring revision | Presented frames | Live recipe |
| --- | ---: | ---: | --- |
| Manual original import | 3 | 9 | original |
| Watch compatible keys | 4 | 155 | keys |
| Undo | 5 | 160 | original |
| Redo | 6 | 256 | keys |
| Committed incompatible duration | 7 | 305 | keys retained |
| Watch compatible recovery | 8 | 365 | keys |

All stages stayed at paused tick 3, Play generation 1, and animation clock
`0.5500000000000002` seconds, with identical playback flags, completion and fade
state. One normal Step reached tick 4 and `0.5666666666666669` seconds. The persistent
script translation increased by the expected 8; the same world remained admitted.
The refusal retained its actual console event, including “animation clip bindings
or duration table changed”; no native error code was invented. Reopening the real
journal recovered revision 8 and the compatible recipe. All six owned children
exited successfully and were reaped.

Final source is `d2add5f1c6feb612cab8d758785ef83eb914263c`, joined with main298
`1ebd80a43c95c49d7416ccef0906e696a1f7fdda`. Source001 CPU compilation failed on an
undeclared direct `incant_types` import; that original log is retained. Source002
uses the existing `incant_doc::Animation` reexport, with no dependency change.
Thirteen Python controls passed on unchanged Python sources. Source002 passed
23 focused Play CPU tests, the editor/headless build, and strict editor all-target
Clippy. A separate four-child CLI control imported all three skeletal fixtures
with a stable asset ID. Those are CPU checks, not window or pixel gates.

The genuine frontend was built at source001 and reused after all 145 frontend
inputs and 19 output identities matched source002. The actual source002 app hash
is `0095081df176442c3161d6339c02546053490aa9c6e8f5bd3c71b1d0ac28244c`;
the headless hash is
`ab5491bf903102aaedaa0c00b6bc9ef0581f506a7247ee7ce39137d5f38aacc1`.
The CPU test executable is separately identified in the manifest.

Window001 ran on macOS 27.0.1, Mac17,9 / Apple M5 Pro, Metal, using a local unsigned
bundle with ID `dev.incant.animated-model-workflow-probe`. Owned PID 33370 completed
before the root's late Finder Open. That later action opened an incidental default
instance, which root closed; it is excluded from the workflow and all pixel claims.
No owned animated-workflow pixels were observed. The unchanged bounds were 120s
for the app, 20s for the startup predicate, and a 180s cooperative execution deadline
plus mandatory cleanup; these are not a hard bound on blocking IO or cleanup.

The [portable manifest](evidence/native-window-animated-model-workflow/manifest.json)
binds raw logs, source copies, commands, fixtures, journal, process ledger, and root
closeout. Executables and frontend assets remain external, identified by hashes.
The verifier never starts an engine, build, RPC, or GPU:

```sh
python3 docs/spikes/evidence/native-window-animated-model-workflow/verify.py
python3 docs/spikes/evidence/native-window-animated-model-workflow/verify.py \
  --originals --binaries --git-source d2add5f1c6feb612cab8d758785ef83eb914263c \
  --source-root /path/to/incant
```

`--git-source` checks the frozen compiler/config/frontend input inventory at that
immutable commit. `--source-root` checks the nine final probe/runner paths only; it
does not assert whole-runtime equality for a later integration.
