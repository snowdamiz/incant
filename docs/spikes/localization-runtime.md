# Localization runtime and typed string tables

This increment implements the nonvisual localization runtime in PLAN revision 3.
The editor remains English-only. Text shaping, bidi layout, font fallback, IME,
localization panel design and Core Sample visual
acceptance remain separate open work; Unicode string output alone proves none
of those rendered behaviors.
The subsequent [translator exchange increment](localization-exchange.md) adds
XLIFF import/export and source/translation argument-contract validation.

## Authored documents and commands

`Project.string_tables` maps stable ULIDs to `StringTable` documents. A table has
an ID, name, source locale and `messages[key][locale]` patterns. Every key needs
its source-locale value. An empty translation is an intentional value. Table IDs
share the project's global identity namespace. `upsert_string_table`,
`remove_string_table` and `set_locale` use the existing command bus, validation,
agent transaction provenance, CRDT history, journal and Undo/Redo. A rejected
suffix rolls back all preceding commands. No file, network, shell or alternate
project mutation capability is added to scripts or the engine agent.

`settings.localization` contains the requested locale, ordered fallback locales
and a pseudo-localization flag. Empty tables and default locale settings are
omitted from canonical JSON, preserving the serialization of older projects.
Nonempty data appears in the generated Project, Command and GameSave schemas;
the CLI also exports dedicated localization schemas and generated SDK types.

Locale tags use ICU's canonical BCP-47 syntax and casing (`en-US`, not `en_US`).
For a missing translation, lookup tries the exact requested tag, its base ID and
successively shorter parent tags, then the configured fallback tags and parents,
then the table's source locale and parents. It does not guess a script or choose
another regional sibling. Exact empty values stop fallback. `LocalizedText`
reports the requested/resolved locales and a missing-translation diagnostic if
fallback was used. An unknown key returns `⟦table:key⟧` and a missing-key
diagnostic; an unknown table or malformed arguments returns a typed error.

## Supported MessageFormat subset

- Literal text, `{name}` substitutions and apostrophe-friendly ICU quoting.
- `{n, number}` using ICU4X locale decimal formatting.
- `{date, date}` with optional `short`, `medium` or `long` style. Input is an ISO
  calendar `{year,month,day}`; ICU uses the locale's output calendar.
- `select` with arbitrary stable selectors, including game-authored gender.
- `plural` and `selectordinal`, CLDR categories, exact `=number` selectors,
  nonnegative integer `offset`, nested arguments, and `#` substitution.
- Every select/plural requires `other`. Unknown styles, duplicate branches,
  unmatched braces, choice/time/currency/percent/skeleton formats fail explicitly.

Literal apostrophes double with `''`; quoting starts before a brace, or `#`
inside a plural. Nested select inherits the surrounding plural's `#`; a nested
plural supplies its own value. Exact selectors compare before the offset;
category selection and `#` use the adjusted value. Numbers are finite f64 values
within JavaScript's safe-integer magnitude. Decimal trailing-zero precision is
not represented by the numeric argument type. Fallback messages use their
resolved locale for both grammar and embedded number/date formatting.

Pseudo-localization accents Latin literals, doubles their vowels, and wraps the
result in `[!! … !!]`. Supplied arguments, numbers and dates remain intact. This
is a text transformation; layout overflow and RTL/CJK rendering require Claude's
future rendered-pixel review.

The parser follows the documented [ICU MessageFormat syntax](https://unicode-org.github.io/icu/userguide/format_parse/messages/)
for the subset above. Formatting and plural rules come from locked ICU4X 2.3.0
crates and their compiled Unicode/CLDR data. This is not a complete ICU
MessageFormat implementation. Unicode and calendar dependency notices are
included in development app bundles and desktop CI artifacts.

## Runtime, saved games and reports

The sandbox exposes `api.localize`, `api.locale`, `api.formatNumber`,
`api.formatDate` and `api.localizationReport`. Each query costs four units from
the existing 256-unit native-query tick budget and checks the script deadline.
Responses are copied JSON values. Localization reads the start-of-tick document;
a successful `set_locale` command becomes visible on the next tick. Timer
callbacks follow the same rule. Replacing tables rebuilds the compiled catalog
before the following callback; unchanged tables reuse it. Catalog preparation
and project serialization occur before the callback deadline, as existing host
preparation does; the callback budget is not a total frame-time guarantee.

Save files retain runtime locale choices and restore them in a new process,
without changing authored files. String-table contents belong to the resource
manifest: a save cannot replace authored translations, assets or script
resources. Author/project and compiled-behavior revision checks still apply.
Hot reload reinstalls the read-only localization capability and prepares the
current catalog at the next tick.

`incant localization-check PROJECT [--locale TAG]` emits deterministic JSON
coverage for all declared keys and exits unsuccessfully when translations are
missing. `--allow-fallback` explicitly permits that exit condition while keeping
`complete:false` and all diagnostics. Invalid locales still fail. The check does
not edit the project or open its journal. Runtime requests for undeclared keys
are reported by `localize().missing`; coverage does not aggregate query history.

## Bounds and verification

Limits are 64 tables, 4,096 total keys, 32 locales per table, 4 MiB combined
pattern/key/locale text, 8,192 bytes and 1,024 nodes per pattern, 16 nested
arguments, 64 branches, 64 arguments, and 64 KiB combined text arguments/output.
Fallback lists are at most 16 tags. Dates require a valid ISO day in years
-9999 through 9999. The CLI/project validates every pattern before publishing
an atomic transaction.

Behavior tests cover cardinal/ordinal rules, exact selectors, gender, offsets,
quoting, nested selection, fallback, empty values, pseudo expansion, localized
numbers and calendars, malformed data, output limits, command rollback,
provenance, journal reopening, Undo/Redo, next-tick switching, hot reload, saved
locale continuity, resource tampering and query limits. The public
`tools/probes/game-localization.py` additionally uses RPC and strict TypeScript
across headless process restarts, including timer-driven English, Russian,
Japanese, Arabic and French-Canadian fallback changes. Platform probes execute
real compiled locale data; their synthetic text assertions are not visual gates.

All 245 Rust behavior tests, 40 explicitly enabled GPU checks, 315 UI tests/build,
five tool tests, Clippy, generated contracts, strict TypeScript and native release
packaging pass locally. The six public gameplay probes pass (localization, saved
games, actions, audio, timers and input). The macOS platform probe executes real
locale formatting; WASM and iOS target checks compile successfully. Hosted CI is
pending, and no physical-device or shaping coverage is inferred from compilation.

On the local Apple M5 Pro, one release run compiled 4,096 keys × two locales
(699,432 JSON bytes) in 10.567 ms. Ten thousand mixed plural/number/date evaluations
averaged 1.817 microseconds per message. These are isolated synthetic timings;
they exclude project validation, command commits, VM serialization and rendering,
and do not satisfy a Core Sample or reference-device performance gate.

A live `gpt-6-astra` run used the existing saved OAuth account without a login or
Keychain prompt. It added a typed English/Japanese table and selected Japanese
through one approved shared-command transaction, then queried the result. Four
steps consumed 18,086 input and 282 output tokens. Independent validation,
coverage checking, transaction provenance inspection and gameplay execution
confirmed the authored values and output `2枚`. This is a live account/tool/runtime
integration check, not a model-authored complete-game or visual acceptance claim.

[Machine-readable evidence](evidence/localization-runtime-2026-10-10.json) retains
probe hashes and results. Phase 1 and full-engine gates remain open.
