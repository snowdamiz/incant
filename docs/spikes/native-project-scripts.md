# Project-attached native TypeScript

The headless native Play, retained RPC and agent Play routes now consume the
selected scene's `Script` attachments. Startup validates the authoring document,
cooks the scene and one bounded union of its TypeScript modules, then compiles
fresh QuickJS bytecode. Simulation uses the native world and typed component
loans. It does not clone the project, parse source/JSON, or run authoring
validation each tick. Scenes without attachments retain the existing trusted
build-embedded behavior. The ordinary scene-only cook still refuses `Script`
instead of silently dropping it.

## Execution contract

`NativeContext.instances` contains the current source's initial attachments,
ordered by native entity identity, with deeply frozen initial props. Each source
updates once per tick in native source-ID order. Shared modules initialize once
for the union. Props use bounded typed native values, null-prototype objects and
captured engine freezing, including keys such as `__proto__` and `constructor`.
The catalog canonicalizes accepted lowercase ULIDs while retaining original
document keys for lookup.

All source callbacks share one numeric loan, structural-command transaction,
clock advance, CPU allowance and capability quotas. Timer IDs are scoped to
their source; retained timer functions cannot reactivate in a different source
or tick. A later callback failure rolls back earlier numeric/structural changes,
timers and logs, and ends the session. An empty source group retires its timers.
These are cooperating behaviors in one world, not mutually isolated sandboxes:
every source receives the same whole-world bulk views.

Initial attachments bind to an ECS entity generation and world incarnation.
Reparenting/archetype movement preserve an attachment; despawn followed by reuse
of the same stable ID never reconnects it. Plain runtime Spawn adds no script.
A scene fingerprint pairs the prepared program to the exact fresh cooked scene;
this detects mixing and is not executable-code authentication. Starting a bound
program without its world, against a different scene, or after world replacement
fails explicitly. Existing unsafe generated-native-build loading remains the
only crossing for precompiled executable bytes; supplied caches are not accepted.

## Startup compiler boundary

The fixed Node process accepts only a bounded source catalog on stdin. The host
chooses its trusted executable, driver, flags, private output and eight-second
deadline. The cleared environment supplies only runtime installation locations,
private temporary paths and a canonical private SWC cache. Canonicalization
avoids macOS's `/var` symlink rejection without inheriting HOME, NODE_OPTIONS,
credentials or project configuration. File handles bound diagnostics; timeout
or error kills and reaps the child before removing its private storage. The
existing no-follow project graph snapshot and strict TypeScript checks remain.

Limits include 64 source entries/modules, 32,768 attachments, bounded recursive
props, one MiB aggregate source and bounded compiler artifacts. These are safety
limits, not demonstrated capacity or frame-time promises. The developer route
requires the trusted checkout's pinned Node dependencies; it reports startup
source parsing truthfully. Authored runtime VMs omit eval/Function, including
indirect and constructor routes.

## Verification and remaining work

264 scoped Rust tests across 45 groups and scoped all-target Clippy passed,
followed by 730 full-workspace release tests across 208 groups and all-target
Clippy. The 73 hardware tests were ignored in that CPU run. The checks include
11 grouped-binding tests, two authoring-cook tests and a real fixed-Node agent
dispatcher test with two sources/three entities, shared imports, lowercase IDs,
ignored hostile `.swcrc`, numeric motion and failed-candidate preservation.
98 Python, 17 Node and 536 UI tests, strict SDK/UI types, runtime graph,
bridge/schema-generation equality, conventions and formatting checks passed.
The production UI build and all 14 normal runtime dependency roots also pass.
[Local verification record](evidence/native-project-scripts.json).

Independent review found and resolved runtime-evaluator availability and
lowercase source-catalog mismatches. The first real Node integration runs exposed
the missing private cache and then its macOS symlink requirement; both failures
are retained as evidence before the passing run. A separate actual generated-Rust
proof passed 16 copied shipping-player processes after removing its source and
build inputs, with checked compiler flags and matching crash symbols.
[Embedding proof](native-project-script-embedding.md).

The rebuilt public CLI then ran two sources on three entities for 20 ticks,
checking final X positions 2, 3 and 4 within 1e-9. A separate actual provider turn
using the saved ChatGPT login and gpt-6-astra completed native `play_run` and three
state queries without changing the fixture. Its tool report proves invocation
success; the CLI report separately retains the numerical observations. No login
credentials or tokens are stored in the evidence.

This does not provide the editor Play controls, source-file editing transactions,
dynamic attachment/prop updates, hot reload, complete script/session saves,
legacy initialState/onTimer semantics, full SDK or a public project export route.
The grouped native API refuses the unsupported legacy hooks. Cross-platform, device/performance and final integrated hosted results remain
outstanding. No phase gate is approved.
