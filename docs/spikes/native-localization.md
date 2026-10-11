# Native localization and retained locale settings

The authored string tables and locale settings now cook into a bounded native
catalog. `context.localization` exposes `localize`, `locale`, `formatNumber`,
`formatDate` and paged `report`; `commands.setLocale` publishes runtime settings
with the existing native numeric/structural transaction. These are native
TypeScript capabilities, including authored multi-file behaviors and the copied
source-free player. The document-backed legacy gameplay path is not extended.

The shared localization implementation retains its ICU MessageFormat subset:
cardinal/ordinal plural rules, explicit-number branches, plural offsets, nested
select/gender, locale-parent and configured fallback chains, source-locale
fallback, pseudo-localization that preserves supplied arguments, missing-key and
missing-translation results, and ICU4X number/date formatting. Locale commands
do not edit the project. All callbacks in one tick read the same committed
settings; successful commands become visible to the following tick. Invalid
scalar, audio or locale commands reject the whole batch. Each submitted locale
is validated even when a later setting supersedes it.

The catalog and parsed patterns are immutable and Arc-owned. Cooking and native
loading validate and prepare them outside updates; a locale change prepares its
coverage once before publication. `report` copies at most 128 cached records in
canonical table/key order; it does not scan every translation on each call.
Coverage describes declared keys. An unknown requested key is reported directly
by `localize().missing`, not accumulated as mutable runtime diagnostics.

The existing native query loan holds only cheap Arc snapshots of localization.
All guard containers are allocated before moving the three native caches; no
world, solver, cache geometry or catalog is cloned per tick. Guest getters and
result allocation run without Rust catalog/queue borrows. Owned null-prototype
results and own array properties prevent inherited setters from intercepting
materialization. Retained methods are permanently revoked after their update.
Parsing/native/materialization errors poison the tick even when caught in guest
code, including recursive getters and errors in later authored source callbacks.

Limits are explicit: 64 localization queries per whole tick, each charged four
units of the shared 1,024-unit native query allowance; 64 KiB of total argument
text/metadata and 256 KiB of returned text/metadata; 64 locale commands within
the shared structural quota. Requests accept at most 64 typed arguments. Report
offset is 0..4096 and page limit is 1..128. Individual output remains bounded by
the existing 64 KiB formatter cap. Existing CPU deadlines bracket native calls;
they cannot interrupt ICU formatting within one synchronous call. Locale-change
coverage preparation is finite work bounded by the catalog, not a measured
reference-phone timing claim.

Scene v10 adds an explicit little-endian length-prefixed UTF-8 localization
suffix, capped at 8 MiB. The decoder checks counts, lengths, UTF-8, canonical
ULIDs/map order, booleans and trailing data before publication, then uses shared
catalog validation: 64 tables, 4,096 total keys, 32 locales per table, 8 KiB per
pattern and 4 MiB aggregate catalog text. Default empty localization preserves
the existing v1–v9 encoding selection. V10 permits an empty navigation prefix;
v9 still requires navigation. SHA-256 detects corruption, not authorization.

World save v3 retains the current locale, fallback/pseudo settings and immutable
catalog in its nested cooked scene, alongside the existing original scene
binding, exact flipbook fraction and clock. Restore validates a complete
candidate and checks both catalog equality and original binding before replacing
the destination. V1/v2 cannot import or silently clear nondefault localization.
Existing physics/audio-used refusals and navigation invalidation remain intact.
This is not a VM or game-session checkpoint: script heap, input, timers,
callbacks, pending commands, audio cursors and physics continuation remain
outside this save contract. Static Wasm has no localization ABI in this slice.

Local aarch64 macOS validation passed 372 Rust tests across runtime, script,
localization, cook, game-runtime, shared Play and headless, with two existing GPU
tests explicitly ignored. Thirteen new tests cover formatter semantics, owned
results, expiry, recursive getters, whole-tick quotas, caught-error and mixed
command rollback, checked binary/save decoding, legacy refusal and two actual
strict-authored modules. All-target Clippy, compiler-disabled/static-module
checking, runtime dependency checks, strict SDK/templates, formatting,
conventions and generated contracts passed. All 62 previous registry files are
byte-identical; the two new report schemas bring the registry to 64 files.

`tools/probes/native-localization.py` authored source, table, locale and attached
entity in one public command transaction, ran 12 native ticks without changing
the authored document, and reopened its checkpoint/journal in another process.
The same checkpoint produced a fresh `shipping` player. After deleting private
build storage and the probe's authored project/TypeScript/journal, three fresh
copied-player processes with a minimal runtime environment reproduced all 12
messages and the expected 140-unit translation. The script asserts plural,
ordinal, select/gender, fallback, pseudo, ICU number/date and missing reports;
replay reports and PCM hashes were identical. The exporter checked target
codegen, authoring-free normal dependencies and absence of source compiler
symbols. This is CPU functional evidence, not font rendering, RTL/CJK pixels,
Windows/Linux execution, phone performance, a complete SDK or a phase gate.

The independent read-only catalog/persistence and SDK/callback reviews found no
actionable blocker. Exact commands, source/log hashes, executable identity,
replay outcomes and review limits are recorded in
[the evidence](evidence/native-localization.json).
