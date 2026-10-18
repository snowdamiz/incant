# Authored Rust/Wasm localization report export

The ordinary authored export and ten fresh shipping-player cases now pass on
this Mac. The original producer at `b79fb1b97ad99951e1579ebda5ea2c5c1d48a529`
successfully built, authored and exported the package, then failed its first
save-state check because the checker expected the older persistence framing.
That failed attempt is retained. A separately reviewed checker correction at
`486303d5e10c7a56596efa8b5b058c5e2176c79d` and a separate ten-process follow-up
verify the same unchanged package, complete audio and saved continuation.
There is no runtime implementation change, GPU/window result, performance result
or complete SDK/hardware parity claim.

The [earlier report qualification](native-wasm-localization-report.md) covers the
ABI, development/static backend behavior and internal callback rollback. This
follow-up adds the ordinary authored import, source, command, journal, export and
copied shipping-player route, following the
[timer](native-wasm-timers-export.md) and
[coroutine](native-wasm-coroutines-export.md) proof boundaries.

## Fixed behavior and independent oracle

The fixture has two canonical string-table IDs, each with 65 numbered keys and
one intentionally empty translation. The committed callback locales cycle through
`en`, `fr-CA`, `de` with explicit `fr` fallback and pseudo enabled, and `zh-Hant`.
Their exact missing-translation totals are 0, 130, 132 and 64. The guests verify
every row's table/key, requested/resolved locale and Translation kind, including
parent fallback, explicit fallback and direct empty-string presence.

Both guests queue the next locale before reading the report, then check that the
entire current settings snapshot remains unchanged. The Rust guest copies a
128-row page, replaces the host reply with the tail page, and reads both retained
copies. Additional pages exercise `(0,1)`, `(1,127)` and `(4096,128)`. An unknown
`localize` key must return Key with no resolved locale and leave prepared coverage
unchanged. Valid authored catalogs require a source-locale translation; unknown
keys are deliberately not fabricated as report rows.

The guest validates every row against its ordinal contract. It publishes the
accepted callback count, report total and an exact 32-bit FNV witness through one
ordinary Transform. Python derives its expected rows independently from authored
catalog values. The TypeScript reference performs the same public operations on
an independently authored behavior. It is temporarily selected with an ordinary
atomic source/attachment transaction, executed by native Play, then undone before
the Wasm export. Whole binary checkpoints are not compared across JS and Wasm;
selected gameplay observations, locale logs and actual PCM are compared.

## Verified public lifecycle

All project edits use the command bus. Raw Rust source, compiled Wasm and a fixed
integer-generated WAV are private ingress files, not direct project mutations.
The public importer captures the Wasm and WAV, including idempotent Wasm reimport.
One transaction creates the tables, settings, two entities and Wasm program.
Undo/Redo and fresh save/journal reopen must preserve the exact selected project.
A mixed rename/noncanonical-locale transaction must refuse without changing the
project, revision or provenance. The temporary TS source is written/read/undone
through the source transaction path; its cook must not modify authoring.

One fresh ordinary exporter builds its current cooker and shipping player, with
the existing dependency, symbol, codegen and source-free startup checks. Owned
ingress, guest build, trusted WABT copy and exporter staging are removed before
the four player processes: 12 ticks, a 5-tick prefix, and two independent 7-tick
continuations. Explicit persistent state is exactly eight bytes with its declared
schema; expected callback counts, full final checkpoint identity and joined PCM
must agree. A fixed looping WAV makes PCM continuity nontrivial without claiming
audio-device or audible-output qualification.

Six input-selected callbacks tentatively write numeric/state/locale/log data and
then ignore invalid offset, zero/oversized limit, undersized destination, or query
quota errors, or return a deliberate callback error. Each source-free process
must fail with its specific typed classification and publish no successful JSON,
PCM or checkpoint. The runtime marks a failed native session terminal after the
valid input/native step; these subprocess controls do **not** claim whole-tick
rollback or inspect hidden failed-session memory. Internal guest-transaction
rollback remains bound to the prior report qualification.

## Bounds and actual evidence

The producer reuses the checkpoint proof's non-reaping managed groups and
append-only observed PID inventory. It admits one fresh target, jobs/Rayon 2,
5,400 seconds total, a 28 GiB launch floor, 6 GiB ongoing floor, 12 GiB target,
8 GiB private scratch, 2 GiB retained output and 8 GiB owned RSS. Native stages
are 180 seconds for trusted SDK/guest compilation, 1,200 for normal headless,
240 for authoring/reference, 3,300 for fresh export (2,400 shipping child), and
300 for the ten player children (30 each). There are no automatic retries.
The small private copied Node dependency closure is separately inventoried and
bounded to 64 MiB before launch; pinned Node/TypeScript/SWC are authoring tools.
No project chooses executables or grants an engine shell capability.

The original producer took 548.282 seconds and exited 1. Guest compilation,
public import/authoring, journal recovery, TypeScript reference, fresh export,
dependency/symbol/codegen checks and source-free startup succeeded. Its first
12-tick player exited 0 with correct gameplay and PCM; the checker then refused
the save's current timer framing. No remaining player cases ran in that attempt.
All 508 observed process identities/groups were confirmed absent afterward.

The saved state contains the current `INCWPS02` envelope, an 80-byte `INCWST01`
declaration/tick/watermark/guest section and a 32-byte empty `INCTMS01` timer
clock. The guest's declared payload remains eight bytes. The correction verifies
this exact framing, empty timer count, row stride and elapsed clock built by
repeated fixed-step addition. It does not change runtime persistence or accept
legacy framing. Thirteen pure controls now pass, including coherently rehashed
wrong clock, count, stride and wrapper cases. The initial fixture used Python's
compensated `sum`, whose rounding differs from repeated runtime addition; that
test fixture was corrected to model the actual clock operation.

The separate follow-up took 1.814 seconds and ran all ten fresh processes:
12 ticks, a five-tick prefix, two independent seven-tick continuations and six
specific refusals. Every coverage row/witness, selected gameplay observation,
whole final checkpoint and all PCM bytes pass. The baseline checkpoint and PCM
are byte-identical to the original successful 12-tick player, and baseline PCM
matches the TypeScript reference. No compiler, export or GPU stage was repeated.
All eleven wrapper/player identities and groups were freshly absent afterward.
Combined elapsed time is 550.095 seconds, not a replacement for the failed
original receipt.

Independent source and saved-data review found no issues. The latter rechecked
105 bound inputs, 62 follow-up files, actual build/authoring records, every player
stream and the full continuation comparison. The original package player SHA256
is `5cbf32940628ab5f57ce9762b6c297c6463e90516932e558c7b0803e2f32a2bd`.
The [retained evidence manifest](evidence/native-wasm-localization-export-20261018/manifest.json)
contains 192 compressed records with compressed and decoded hashes, including
the failed producer, separate follow-up, raw reports/PCM/save and review receipts.
Executables and compiler storage remain external and are identified by hashes.
All ten local development checks passed before the narrow checker correction;
the corrected checker has its separate thirteen-control result, and the entire
tools suite passes again after that correction.

Producer tools: `tools/probes/native-wasm-localization-export.py` and
`tools/probes/native_wasm_localization/`. Data controls:
`tools/tests/test_native_wasm_localization_export.py`. Frozen packet and failed
preparation records are retained under
`~/.codex/incant-evidence/native-wasm-localization-export-001/`.

The restricted runtime PATH and deletion of owned source/build inputs establish
a source-free package demonstration, not an OS filesystem sandbox. Process-group
ownership assumes trusted tool children retain their group; periodic sampling
does not prove observation of every transient descendant. No phase gate passes
from this scoped proof.
