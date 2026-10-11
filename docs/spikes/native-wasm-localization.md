# Native Wasm localization

`incant_localization_v1` adds real native catalog queries and atomic runtime
locale changes to both the Wasmtime development host and linked wasm2c host.
The registry appends four imports, preserving existing opcodes 0–17 and all
numeric, input, playback and log layouts:

| Opcode | Import | Result |
| --- | --- | --- |
| 18 | `locale(epoch)` | Latest copied result byte length |
| 19 | `localize(epoch, ptr, len)` | Latest copied result byte length |
| 20 | `read_result(epoch, ptr, capacity)` | Complete copied result byte length |
| 21 | `set_locale(epoch, ptr, len)` | No scalar result |

Epochs are `i64`; offsets, lengths, capacities and scalar results are `i32`.
An older host refuses these imports explicitly. Each query replaces the one
private result; an exact-epoch read copies it synchronously into initialized
guest memory. Reads before a query, after an epoch, or with an undersized or
out-of-range destination fail without writing a prefix. The buffer is discarded
at begin, accepted finish, abort, authorized localization failure and replacement.
The SDK immediately copies and validates a query's response; its returned text
borrows caller-owned scratch and can safely remain an old observation afterward.

`Tick::locale`, `Tick::localize` and `Tick::set_locale` use allocation-free
`incant_module::localization` types and caller-owned request/result scratch. The
runnable [Rust sample](../../sdk/rust/samples/localization.rs) formats Unicode,
plural/ordinal/select and dates, inspects missing-key metadata, queues locale
changes and demonstrates that subsequent queries in the same callback still
observe the original committed locale. Ignoring an active SDK method's returned
error poisons the callback, just as existing numeric and logging methods do.

Tick start takes only the existing three-Arc `NativeLocalization` snapshot.
Message arguments are unique named Text, finite Number or CalendarDate values;
table IDs are canonical 26-character ULIDs and keys/names use the shared ASCII
key domain. Native shared validation and ICU rules determine canonical BCP-47
tags, actual calendar validity, plural/ordinal/select, dates/numbers inside
messages, configured/source fallback, pseudo text, and missing-key/translation
metadata. No controls are stripped from localization argument text. A missing
table is an error; a missing key is the existing placeholder with `Key` metadata.
Requested/resolved locales accompany the outer result and apply to its optional
missing record; the wire does not duplicate those two strings.

`set_locale` validates every submitted setting before queueing the existing
`StructuralCommand::SetLocale`. The final valid setting wins after successful
native acceptance. It never edits an authoring project or independently mutates
the world/catalog. Numeric, structural, locale and log work publish together;
invalid commands, numeric rejection and traps publish none of that staged work.
Successful development reload keeps the committed world locale and diagnostic
history/clock; failed candidates leave the current instance unchanged. Static
NativeSession observers receive actual localized guest logs and refuse all
observation after terminal failure. No static checkpoint support is added.

## Copied protocol and limits

All multibyte fields are little-endian. Frames begin with eight magic bytes and
an explicit `u32` version 1. Text is `u32 byte_length` followed by exact UTF-8;
IDs use exactly 26 canonical bytes. Counts are checked before walking fields,
lengths before copying/allocating, and complete frames reject trailing data.
The safe no_std ABI codec is shared by host and guest; native semantic validation
is still authoritative. Requests use `INCLQR01`: table, key, argument count,
then name/tag/value records. Tags 0/1/2 are Text, `f64`, and an `i32` year plus
month/day/two zero reserved bytes. Settings use `INCLST01`: locale, fallback
count/texts, then `u32` pseudo exactly 0 or 1. Results use `INCLRS01` plus a
`u32` kind: 0 settings or 1 localized text/requested locale/optional resolved
locale/optional missing table+key+kind. Option tags and reserved bytes are strict.

Logical quotas match native TypeScript: 64 localization queries and 64 locale
commands per callback, 64 KiB aggregate request text/metadata, 256 KiB aggregate
returned text/metadata, and 64 KiB per formatted text. Each query costs four
units of the 1,024-unit allowance; localization is currently the only Wasm query
namespace consuming that counter. Numeric/input/playback imports retain their
existing common call/transfer quotas. Locale commands also consume the existing
1,024-command structural allowance. There are at most 64 arguments, 16 distinct
fallbacks, 128 bytes per locale/key/name, finite numbers within the JS-safe
integer magnitude, and shared valid calendar dates in years -9999..9999.

Wire limits are separate: 66,560 bytes per request/result and 2,364 bytes per
settings command. This includes frame/length/tag overhead; it does not enlarge
logical quotas. Each input/output copy also consumes the shared 128 MiB transfer
allowance, with existing 32 MiB memory and 65,536-call limits. Active localization
calls using the correct epoch poison the entire batch even when they fail in
common call/memory/transfer ingress before localization dispatch. Unrelated
direct capability calls and stale/inactive tokens retain their previous error
semantics; actual VM import errors still trap/abort normally. ICU calls remain
synchronous bounded native work, without a preemptible formatter deadline claim.

## Local validation

The actual Rust guest and trusted WAT controls run through Wasmtime 49.0.2 and
pinned WABT 1.0.42 generated C. Tests observe localized outputs, copied/replaced
results, unaligned reads, next-callback settings, preserved reload state,
missing metadata, mixed rollback, native acceptance failure, and actual
call/memory/transfer/logical/command quota boundaries. The guest-library cache
now receipts all recursively compiled ABI/SDK sources and rebuilds both dependent
libraries when a codec module changes. Its focused control fails against the
unchanged main102 helper and passes after this correction. The existing
`shipping-wasm2c.py --conformance-only` command prepares all new fixed guests.

Exact frozen source, tool, compiled guest, prepared C and command/log hashes are
in [the evidence](evidence/native-wasm-localization.json). These are local macOS
CPU checks. Standalone number/date formatting, catalog editing, report paging,
VM/checkpoint migration, complete SDK, Windows/Linux/browser execution,
graphics, devices and performance gates remain outside this slice. Runtime
source compilation and executable loading routes remain unchanged.

## Current-main integration

Reviewed source `7c66106` merged actual main104 (`3da7185`) cleanly as
`6d0d59b`. All 35 reviewed non-STATUS files and 378 prepared guest/C artifacts
match their frozen identities. [Independent source review](evidence/native-wasm-localization-root-review.json)
reported no actionable finding; [integrated evidence](evidence/native-wasm-localization-main104.json)
records 74 actual Wasmtime/static/SDK tests, 106 ScriptHost/GameSession library
cases and five checkpoint integration tests. Twenty tooling tests, combined
all-target Clippy, compiler-disabled static checking, generated SDK, normal
dependency guards, formatting and conventions pass. Existing static checkpoint
refusals remain in force. No GPU, provider or physical-device result is claimed.
