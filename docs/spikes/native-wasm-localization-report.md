# Prepared localization coverage in the Rust/Wasm SDK

Rust/Wasm gameplay can now read the same prepared missing-translation report as
TypeScript. `Tick::localization_report` returns a borrowed, copied page containing
the total count and canonical table/key entries with requested/resolved locales
and missing kind. The report observes the callback's committed catalog/settings
snapshot. Queuing a locale change does not change queries until the next callback.
Unknown runtime keys remain per-query `localize` results; report reads do not
accumulate mutable diagnostics or edit authored string tables.

The additive `incant_localization_v1.report` import is opcode74 with scalar
epoch/offset/limit parameters. Existing imports and layouts are unchanged.
Result kind3 uses the existing versioned localization reply/copy channel; older
kind-specific decoders reject it. Offsets are 0–4096, pages contain at most128
rows, and the existing66,560-byte result bound accommodates the largest legal
page. Decode validates the entire frame, order, uniqueness, UTF-8, field bounds,
ULIDs and missing metadata before exposing an iterator.

Reports share the existing64-query, four-unit-per-query and256KiB logical-output
limits. Logical byte charges match TypeScript. Copies consume the common transfer
budget and refuse insufficient/out-of-bounds destinations without partial writes.
Authorized failures invalidate the complete callback, including prior transforms,
locale commands and logs; stale epochs cannot poison a new callback. The safe SDK
also latches preflight errors if a guest ignores their returned `Result`.

Source `ec642789f61f7f0342f028b6db5ea18a694231f4` passes23 ABI/runtime controls
across reports and the existing localization/formatting interfaces, plus strict
ABI/SDK/runtime lint. Six actual compiled Rust guests pass Wasmtime and wasm2c
qualification: prepared canonical pages, next-commit locale visibility, retained
copies, save/restore and development reload; ignored offset, limit, capacity and
quota errors; and explicit callback failure. Invalid namespaces/signatures are
refused before execution. Three parity tests and the registry test pass, followed
by strict development-host lint. Independent source review found no issue.

[Qualification identities and commands](evidence/native-wasm-localization-report.json)
retain both local owners and actual terminal results. The27 native controls are
functional CI-profile checks on this Mac. They do not establish shipping cost,
an authored shipping export for this query, full SDK parity or phase completion.
The shared conformance producer now builds these guests in the hosted suite.
