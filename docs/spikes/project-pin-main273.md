# Project pin migration on main273

This composition adds the accepted host-bound project-file protocol to main273,
while retaining linked-prefab authoring and engine compatibility. A trusted host
can open a legacy unpinned project, review its exact-current metadata pin, apply
it through `incant_cmd`, recover interrupted writes, and undo/redo the exact file
change. Original bytes are backed up create-only. This slice adds no CLI, RPC or
editor endpoint and no older-format conversion.

The producer is `999f0cde0deeaf38371bd860fa798c3beec1388d`; main273 is
`0bc6823ace3dd88fea436b4954e7f081fd35c6e7`. The initial tested composition is
`31e86a437a81897cdc59a9cd8efc2529d8b0b988`. Eleven of twelve producer paths were
exact; the command implementation retains the already-main prefab dispatch,
validation and budgets. Seven producer build inputs differed at that point,
all explicitly recorded. Cargo manifests, dependency lock and toolchains did
not change. A new integration control proves pin/apply/reopen/undo/redo retains
linked instances and component overrides, then exercises the ordinary shared
prefab revert command without silently rewriting the project file.

## Executed qualification and retained generations

On the initial composition, all nine commands passed: 127 source/document/command
cases, two headless cases, strict four-package Clippy, regeneration and exact
comparison of 74 schemas, SDK freshness/types, formatting, conventions and diff.
The test counts overlap earlier producer controls. Thirty-five executable copies
were retained immediately; they remain tied to this source generation.

Checkpoint33 later exposed Windows error33 in the existing engine-version tests:
a separate `fs::read` tried to read a journal still held by the bus's exclusive
lock. Its original exact-head hosted log and job receipt are retained. These
ordinary persistent-bus tests now close/read/reopen with the same initial
project, preserving revision, project, history, refusal and byte comparisons.

The new host migration controls received the same audit. Ephemeral file reviews
must stay live, so their journal observations use the already-open lock-owning
handle and restore its cursor. The sparse-capacity test also uses that handle.
A full artifact inventory is compared before opening and after refusing the
review and closing the bus. The unlink/recreate-same-name control is Unix-only:
Windows retains a deleted node while the review pins it. The rename/recreate
identity refusal remains cross-platform. The sole journal accessor is
`cfg(test)`; no production lock, deadline or admission rule changed.

Generation003 did not execute tests: Cargo reported a fresh `incant_doc` artifact
that lacked the prefab API present in the unchanged frozen source. The full
compiler failure is retained. A target-local `cargo clean -p incant_doc` in the
exclusively held font cache forced the correct source to rebuild; no source fix
is inferred from this cache observation. Generation004
`5415aee33a3c2efc8c11f89aa457378822965916` passed three engine-version cases, the
one prefab/file integration case, and fifteen host unit cases. Its lint run then
caught an unnecessary test-only `mut`. Generation005
`183f9f8a17ccd1d5fed8a9654f1a586b9bea3c1e` removes only that qualifier and passes
the four overlapping integration cases, strict command-package Clippy, formatting,
conventions and diff. Forty current executable copies cover these recorded
attempts. All managed children were reaped and the font target released.

The producer packet is unchanged: 124 lossless records, 77 payloads, 3,542 input
identities and 25 executable copies; its 87 passing cases and earlier real Abort
encoding failure remain explicitly producer-scoped. Current composition freezes
bind 3,559 inputs. The final ten producer-input exceptions include the three
reviewed test-only successor paths. The archive also preserves the unsuccessful
local cache and lint attempts. No tests from an earlier generation are relabeled
as a new execution.

## Review and boundaries

Root accepted producer source/execution (`47051d83…`). Independent successor
source review (`813206f3…`) accepted the owned-handle observations and ordinary
bus lifecycle, with the subsequently added Unix restriction required. Final
current-main packet review is pending; there is no PR or merge claim yet.

Checkpoint33 stays failed and frozen until all original workflows finish. Local
Mac success does not claim repaired Windows execution. A later reviewed main
merge and actual successor required checks are still necessary. Application
binding, older-format conversion and phase/release approval remain separate.

Replay without compiling or executing test binaries:

```sh
python3 docs/spikes/evidence/project-pin-main273/verify.py --verify-only --current --originals --local-binaries
```

Omit `--originals` and `--local-binaries` for portable embedded/Git verification;
omit `--current` when verifying historical source after further development.
