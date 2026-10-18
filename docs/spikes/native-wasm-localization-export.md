# Authored Rust/Wasm localization report export

This is a prepared, unexecuted shipping proof on main401
`843d6d4b3838d35a158a5178ea5a9fd6e010a7e5`. The authored Rust guest, independent
TypeScript reference and data-only oracle are implemented. Native compilation,
public export and player execution require the separately reviewed CPU grant.
There is no new runtime implementation, GPU/window route, performance result or
complete SDK/hardware parity claim.

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

## Planned public lifecycle

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

## Prepared bounds and current evidence

The proposed owner reuses the checkpoint proof's non-reaping managed groups and
append-only observed PID inventory. It admits one fresh target, jobs/Rayon 2,
5,400 seconds total, a 28 GiB launch floor, 6 GiB ongoing floor, 12 GiB target,
8 GiB private scratch, 2 GiB retained output and 8 GiB owned RSS. Native stages
are 180 seconds for trusted SDK/guest compilation, 1,200 for normal headless,
240 for authoring/reference, 3,300 for fresh export (2,400 shipping child), and
300 for the ten player children (30 each). There are no automatic retries.
The small private copied Node dependency closure is separately inventoried and
bounded to 64 MiB before launch; pinned Node/TypeScript/SWC are authoring tools.
No project chooses executables or grants an engine shell capability.

The 12 pure Python controls currently pass, including malformed/partial evidence,
row-field/order changes, clock/schema/count mismatches, PCM corruption and
compiler-override refusals. Strict TypeScript source checking passes. An initial
negative test mistakenly treated a higher valid epoch watermark as invalid; its
failed source/result is retained externally, and the corrected test supplies a
watermark below the saved tick without changing the oracle. These are checker
and source results only. Rust guest compilation, every actual native assertion,
typed stderr classification and shipping lifecycle remain pending.

Producer tools: `tools/probes/native-wasm-localization-export.py` and
`tools/probes/native_wasm_localization/`. Data controls:
`tools/tests/test_native_wasm_localization_export.py`. Frozen packet and failed
preparation records are retained under
`~/.codex/incant-evidence/native-wasm-localization-export-001/`.

The restricted runtime PATH and deletion of owned source/build inputs establish
a source-free package demonstration, not an OS filesystem sandbox. Process-group
ownership assumes trusted tool children retain their group; periodic sampling
does not prove observation of every transient descendant. No phase gate passes
from this prepared packet.
