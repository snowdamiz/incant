# Registered Rust source authoring

This source foundation adds stable registered Rust entry IDs and shared atomic
source editing. It is not the complete `module.read/write/build` agent workflow:
the dedicated tool adapters, source-ID-to-build resolver, checked candidate
installation and transactional first-package creation remain follow-on work.
No new compiler, shell, runtime authoring dependency or module selection is added.
The foundation is CPU-qualified as described below. That qualification does not
complete the eventual source-author/build/apply workflow.

`Project.module_sources` maps globally unique ULIDs to `RustModuleSource { id,
name, path }`. This optional map defaults to empty and is omitted when empty, so
legacy projects retain their canonical bytes. Registered source IDs are distinct
from compiled Wasm asset IDs and scene programs. Registration does not select,
compile or execute anything.

`incant_cmd::Command` admits `UpsertModuleSource`, `RemoveModuleSource` and
`WriteModuleSource`. The write carries the stable module ID, exact observed
`SourceVersion` and replacement UTF-8 text; execution also requires the observed
project revision. `CommandBus::read_module_source` returns the registered path,
revision, text or missing state, content version and whether current bytes match
the latest tracked source edit. It does not reconcile external changes. These
operations use the same trusted project root, exclusive persistent journal,
source recovery guard and transaction provenance as TypeScript source edits.
There is no direct editor/agent filesystem mutation route.

Exactly one source write is allowed in a transaction, across both TypeScript and
Rust. Mixed TS/Rust or multiple Rust writes refuse before file or document
publication. Registration, one write and ordinary document changes can be one
reversible transaction, with registration resolved from the final document.
The existing Prepare/install/Commit/Abort protocol, exact preimages, Undo/Redo,
source epoch and recovery paths are reused. `SourceEdit.kind` defaults to
TypeScript and is omitted for TypeScript records. Rust records explicitly use
`"kind":"rust"`; the historic `script_id` wire field carries either kind's
stable source ID. Old journal records do not gain a required field or a new
version. Old binaries fail closed on new Rust records.

The initial scope edits files in an **existing nested package directory**. Paths
must be project-relative, ASCII, at most 1,024 bytes, at most eight components,
and at most 128 bytes per component. Components begin with an ASCII letter,
digit or underscore and otherwise contain letters, digits, `_`, `-` or `.`.
Rust entries end in lowercase `.rs`; hidden names, traversal, Windows device and
trailing-dot/space aliases, `build.rs`, symlinks and hardlinked source files are
refused. Root-level `main.rs` is explicitly refused: the current worker package
snapshot must not include unrelated project-root JSON/assets. A future adapter
can resolve `movement/main.rs` as package `movement`, entry `main.rs`, or
`movement/src/lib.rs` as package `movement`, entry `src/lib.rs`, without accepting
the project root. The package still has to satisfy the worker's complete source
snapshot and Rust-only recipe; registering a path does not certify that package.

Reads and writes admit at most 64 KiB of Rust UTF-8 bytes. TypeScript keeps its
existing 256 KiB bound and path admission. Missing parent directories are refused
and never created by `SourceStore`; no recursive deletion or move is added.
Removing registration or changing its path leaves previous bytes intact. Empty
text is an authored source value, not file deletion. External edits cause stale
write/Undo/Redo refusal rather than being overwritten.

Transactional creation of the first package remains required follow-on work,
including ownership and reversible cleanup of directories; it is not permanently
excluded from the intended workflow. The current foundation can create a new
source file only beneath an already admitted existing package directory.

Compatibility review covers serde/CRDT materialization, global ID and path alias
validation, save/reopen, legacy journal decoding and generic destructive `doc_patch`
approval. The UI's existing approval-description renderer identifies the new
operations without implying build authority or file deletion. Fresh schemas and
TypeScript declarations were generated through the ordinary headless schema
export and SDK generator, then checked against the published files. The four
existing package-based module-build tool descriptors remain byte-equivalent.

Authored controls cover legacy canonical/wire bytes, typed schema admission,
registration/write order, exact provenance, external CAS conflicts, mixed-write
refusal with no published prefix, source bounds/links, metadata-only removal,
save/reopen and real subprocess recovery at every existing durable boundary for
apply, Undo and Redo. The subprocess fixtures are self-contained and need no
external project, compiler, device, window or account.

## CPU qualification, 2026-10-18

The original feature is `e1b4d8dd`; composition `e2c7e8f1` preserves its 18 files
and joins main425 plus the separately qualified module-build service. Final
producer `d560b14a` adds one test-only metadata exclusion: `RustModuleSource` is
authoring metadata, not an entity component. The exhaustive component-reference
audit and unknown-component refusal remain intact. Generated-only successor
`f74cd25b` contains ten ordinary generator outputs; it does not change the
qualified implementation.

The retained Mac CPU runs establish **72 passing parent tests**: 18 source-store,
four document, 33 command-library, eight registered-Rust integration, five legacy
TypeScript integration, two generic agent-patch and two module-service mock
controls. Source/document results were carried from their successful native
stages. The Rust/TypeScript integration executables were copied from the successful
`e2c7e8f1` compilation and first executed after the unrelated metadata-test fix;
they are not relabeled as freshly compiled at `d560b14a`.

Every failed supervisor result remains failed:

| Attempt | Retained result | Correction and continuation |
| --- | --- | --- |
| CPU002 | All 18 source-store tests passed; the supervisor incorrectly counted only 16 inline results when two crash-recovery parents printed child harness headers. | A serial parent-ownership parser, with 20 pure controls and independent raw replay, qualified the original 18 results without rerunning them. |
| CPU003 | Four document controls passed; the command suite passed 32/33 and failed its component-metadata inventory. | The one-line test-only exclusion was committed; all 33 command tests then passed. |
| CPU004 | All 50 remaining behavioral tests passed; strict Clippy was cancelled after the five-second process-sampling command timed out. | Normal subsequent sampling was retained; memory/ownership checks were unchanged. Only unfinished lint and generation continued. |
| CPU005 | All 12 remaining managed calls passed, including strict all-target Clippy for the six affected crates, fresh headless build, schema export, SDK generation/check/typecheck, seven schema-comparator controls, 32 approval-text controls, formatting, conventions and diff checks. | No behavioral tests were repeated. |

The copied headless executable generated 82 schema/tool files plus the TypeScript
declarations (83 output files). The expected publication delta is two new schemas,
seven changed schema/tool files and one changed declaration file. All four legacy
module tools (`module_source_read`, `module_build`, `module_build_status`,
`module_build_cancel`) retain their exact descriptors. After applying the ten
generated files, six further managed checks passed: published schema parity,
SDK freshness, SDK typecheck, actual editor `tsc -b`, conventions and diff checks.
These consumer checks did not access the shared Cargo target.

The native recipe used pinned Rust 1.99.0 for aarch64 macOS, locked/offline profile
`ci`, two jobs and Rayon threads, with the existing renderer/sha2 optimization
overrides. All earlier charges and artifacts were retained. CPU005 closed with
285 observed PID identities and 12 groups absent, 108 protected external artifacts
rehashed, target allocation/retained peak 14,705,061,888 bytes below its 18 GiB cap,
and cumulative history 6,250.465674 seconds after cleanup / 6,709.470318 seconds
including earlier deleted-target history. These are resource-accounting totals,
not performance results. The target lease was released before consumer checks.

## Portable replay and limits

The [portable packet](evidence/registered-rust-sources/manifest.json) retains
source/plan/runner bindings, all four attempt outcomes and raw logs, independent
reviews, copied-executable identities and generated/consumer receipts. Its
data-only verifier checks exact parent membership and failure retention; it does
not execute archived code, build tools or the engine:

```sh
python3 -B docs/spikes/evidence/registered-rust-sources/verify.py
python3 -B docs/spikes/evidence/registered-rust-sources/verify.py --originals --binaries --source-root /path/to/matching/checkout
```

The default replay needs only repository data. Optional checks require the retained
external originals, nine copied executables, or a matching source checkout. Full
historical source manifests are externally hash-bound; `--source-root` checks the
260 listed implementation/tool inputs and all 83 generated outputs, not every
historical documentation file. Binaries and whole-tree manifests are deliberately
not embedded. Recorded process absence is an observation at closure, not a claim
about the current machine.

This is source-authoring/recovery and contract evidence on macOS. It is not a
Windows execution result, a live model-provider test, compiler/candidate-install
proof, shipping-runtime qualification, visual acceptance or performance gate.
Transactional first-package creation, registered-ID adapters and checked candidate
application remain separately qualified parts of the intended complete workflow.

## Integrated development checks

Integration producer `5256a8df` passes all ten ordinary development checks,
including 1,041 UI tests, in 111.881 seconds (actual session `6240`, terminal exit
zero). Source contents stayed unchanged; all 258 observed identities and their
groups plus the wrapper were independently absent afterward. The first integrated
run remains failed: five Python lifecycle fixtures expired before readiness. The
[test-only synchronization repair](native-process-cleanup.md#separate-fixture-startup-from-execution-deadlines)
preserves the production deadlines and passes 48 focused controls.
The [integration record](evidence/registered-rust-integration.json) binds exact
commands, source, terminal result and retained raw streams. Documentation added
after this producer does not change its tested source identity. These development
checks do not establish rendered performance or a phase gate.
