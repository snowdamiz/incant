# Core Sample 10,000-script workload

`samples/core/stress.py` adds 9,999 invisible Transform/Script entities to a fresh
copy of the accepted, fully bound Core Sample. The existing gameplay controller
remains its one-instance source; the second source updates all 9,999 attachments
through borrowed typed bulk views. There are exactly 10,000 Script attachments
across two sources. No render, physics, audio or UI components are added. The
Core controller, assets, strings, settings and existing entities remain exact.

The helper uses public `incant_cmd` source/entity transactions, with 256 entity
commands per transaction. Every transaction is checked through undo and redo,
then saved/reopened. Project content is never patched on disk. The helper retains
compressed raw RPC bytes, request/response hashes and bounded child exit receipts.
The saved project and sorted membership manifest bind the source IDs, entity IDs
and ordinals. The original input tree is rehashed before/after authoring.

The bulk script writes closed-form numeric positions for every owned row. One
sentinel publishes actual row count and aggregate readbacks of its written X/Z
columns. Native observation checks use independent scalar formulae at ticks
1, 63, 64 and 65, including phase wrap. The original Core entity states, logs
and offline PCM are compared with an unchanged reference. Node controls examine
every typed row and unchanged non-translation columns, plus malformed membership
and an omitted-write mutation; these are CPU model controls, not native evidence.

The original Core input omits `physics_mode` and is deterministic.
`samples/core/stress_mode.py` creates a **separate** fast-mode base with one
public `set_physics_mode` transaction, undo/redo/reopen and an exact settings-only
diff. For the 10k copy, `--stress-receipt` binds the already qualified deterministic
extension; only the public mode edit is repeated. Prior authoring journals are
retained in their original evidence but excluded from this fresh copy.
The old deterministic headless binary must not be used to claim fast execution.

Example (all outputs must be fresh, outside the protected input directory):

```sh
python3 -B samples/core/stress.py ORIGINAL/core.json DET_OUT --binary HEADLESS --input WIN_INPUT
python3 -B samples/core/stress_verify.py ORIGINAL/core.json DET_OUT/authoring/core.json CHECKS --binary HEADLESS
python3 -B samples/core/stress_mode.py ORIGINAL/core.json FAST_BASE --binary HEADLESS
python3 -B samples/core/stress_mode.py DET_OUT/authoring/core.json FAST_OUT --binary HEADLESS --stress-receipt DET_RECEIPT
```

`stress_verify.py` executes only 65 headless ticks per reference/stress case with
no GPU submission. It does not change VM, physics or script budgets, and failures
remain failures. The helper's authoring RPC deadlines concern tool/process
lifetime, not gameplay budgets. A retained binary may qualify public authoring
and native numerical behavior only under its exact source/compiler identity.
Fresh shipping export, source-free execution, current fast-mode behavior and
Mac performance measurements are separate evidence. CPU timing is not a 60 fps
or GPU/frame-presentation claim. No appearance acceptance is supplied here.

## Retained producer results

- Ten Python authoring/lifecycle controls pass; strict TypeScript and the Node
  all-row/mutation controls pass. No Rust build was started for this slice.
- Actual retained headless binary `7b75efd9` (source `2489325e`) completed 41
  public authoring transactions and every undo/redo snapshot. Initial full
  journal reopen exceeded the 60-second tool deadline and was killed/reaped;
  a separate 180-second read-only reopen succeeded against unchanged files.
  This authoring deadline is not a gameplay budget change.
- The deterministic 65-tick reference and 10k runs pass all 28 numerical checks.
  All 46 original entity readbacks, logs and PCM match exactly. The Core assets
  and authored input trees remain byte-identical after native Play. The reused
  compiler freshly compiled both source groups; this is historical-engine CPU
  correctness, not current shipping equivalence or a performance measurement.
- Separate normal and 10k fast-mode inputs pass public settings-only edit,
  undo/redo/save/reopen. A redundant fast reauthoring attempt was intentionally
  interrupted/reaped when the smaller one-edit path was selected; its raw
  evidence is retained, not counted as a passing authoring run.
- The final fast input contains 10,045 entities and exactly 10,000 attachments.
  It is prepared for a fresh shipping run, with no fast/device/GPU result claimed.

Local raw evidence is `~/.codex/incant-evidence/core-script-stress-001`.
The portable manifest retains compact checks, frozen helpers, final project and
membership data; large authoring journals/RPC transcripts and reused assets are
external hash-bound evidence, explicitly identified by the verifier. It does not
claim those external payloads are embedded in the repository archive.
