# Translator exchange through shared history

The CLI exports typed string tables as UTF-8 XLIFF 2.1 and imports translated
targets through `incant_cmd`. This implements the nonvisual translator exchange
workflow in PLAN Sections 5 and 6.13. The graphical localization panel, shaped
text, bidi/font fallback, IME and Core Sample visual/device gates remain open.

## Export, translate and import

```sh
incant localization-export game.incant.json TABLE_ULID ja game-ja.xlf
incant localization-import game.incant.json game-ja-translated.xlf
incant localization-check game.incant.json --locale ja
```

Export takes one table and a target locale different from its source. It writes
a complete temporary file and publishes a new destination without overwriting
an existing path. Table ULIDs become XLIFF file IDs and stable message keys become
unit IDs. The `original` attribute contains the table's display name and is
never interpreted as a filesystem path. Whitespace, XML characters and carriage
returns round-trip. Missing targets are omitted; present empty targets remain
intentional empty translations.

A translator can return a subset of units, and multiple tables with the same
source/target locales can be combined in one document. Each returned source must
match the current source pattern exactly. Unknown table/key IDs, changed source
locales, stale source text, malformed patterns or invalid argument contracts
reject the whole batch before any project edit. Missing targets leave existing
values alone; an empty target writes an empty value. Segment state does not gate
import: every present target is imported regardless of its workflow state.

`PreparedTranslations` captures the project and revision, validates all targets
and the resulting project, then constructs ordinary `UpsertStringTable` commands.
Commit rechecks the full snapshot and revision before one shared command-bus
transaction. A stale prepared batch cannot overwrite concurrent changes.
No-op imports add no history. The CLI attributes edits to `import/xliff`, uses the
project's usual journal, and saves canonical JSON. Undo/Redo and durable reopening
retain the whole multi-table transaction. The shared service is available to
future editor controls; there is no alternate project mutation path or new shell
capability for the engine agent.

Every translation's argument names and kinds are checked against the source,
including nested branches. Translations may omit source arguments or render a
typed argument as plain text. They cannot introduce unknown names or require a
number/date/select value where the source accepts a different kind. This is a
global pattern contract, not branch-by-branch control-flow analysis; a caller
must still supply the arguments needed by its selected translation branch.
Catalog validation applies the same rule to manually or agent-authored tables.

## Bounded XLIFF profile

The reader accepts XLIFF 2.0 and 2.1 using the standard core 2.0 namespace,
including prefixed elements, groups, comments, translator notes, CDATA and XML's
five predefined/numeric character references. Each unit has exactly one segment
with one source and an optional target. MessageFormat remains text inside those
elements. This follows the [OASIS XLIFF 2.1 core structure](https://docs.oasis-open.org/xliff/xliff-core/v2.1/os/xliff-core-v2.1-os.html);
it is a bounded text exchange profile, not a complete XLIFF schema validator or
a claim of compatibility with every CAT tool. No external CAT product has been
verified in this increment.

Inline codes, extension modules, multiple segments per key, unknown attributes,
nontranslatable units, DTDs, processing instructions and custom entities fail
explicitly. The XML parser never fetches external resources. It requires UTF-8
XML 1.0, validates characters and namespaces, and rejects duplicate identities,
ambiguous fields, misplaced text and incomplete documents. The pinned parser is
quick-xml 0.42.0; its MIT notice ships with development bundles/desktop artifacts.

Limits are 32 MiB encoded input, 4 MiB decoded source/target text, 64 files,
4,096 total units, 8,192 bytes per source/target, 20 element levels, 200,000 XML
events, 16 attributes per element, 512 bytes per recognized attribute value and
32 active namespace bindings. Existing catalog limits also apply to the final
project. Export validates its own result against the import profile so it never
silently emits an unsupported document. Very escape-heavy documents can reach
the event limit before the byte limit and fail explicitly.

## Verification

Behavior tests exercise character/whitespace preservation, empty and missing
values, namespace prefixes, metadata, stale sources, argument mismatches,
hostile XML, duplicates, resource bounds, command atomicity, stale snapshots,
provenance, durable Undo/Redo, export publication and CLI failures.

`python3 tools/probes/localization-exchange.py artifacts/exchange-example` uses
only public CLI/RPC for project changes. It exports two tables, simulates a
translator editing XML, rejects invalid suffixes and stale source text without
changing project/journal bytes, imports one transaction, reimports without
history changes, and verifies exact Undo/Redo across process restarts. A strict
TypeScript behavior switches to Japanese and consumes the imported values,
including an intentional empty translation and source fallback, while authored
files remain unchanged. This is a synthetic translator workflow, not a live
human translation or rendered-text acceptance check.

The public probe runs in source checks and Windows/Linux desktop CI. Six-platform
smoke code additionally exports and parses a real Japanese table using the same
Rust library. Local/hosted results and binary hashes are retained in the
[evidence file](evidence/localization-exchange-2026-10-10.json). No phase gate is
claimed by this increment.

Local verification passes 254 Rust behavior tests, 40 explicitly enabled GPU
checks, 315 UI tests/build, five tool tests, Clippy, generated contracts, strict
TypeScript and native release packaging. The new public workflow and existing
localization/save probes pass. macOS executes the XLIFF smoke check; WASM and
iOS target checks compile. Hosted results are pending at this checkpoint.

Merged in [PR #34](https://github.com/snowdamiz/incant/pull/34) after all 12 checks passed on `37246330f930628713d19fba1130f4feb011026b`. Merge `106a4cc705a6293083bdccc2cc2be3eb5e0325ff` has the same tree (`848bbc12d124b6e5bb6e37825858557d97faf2df`) as that tested head. Deferred release gates remain open.
