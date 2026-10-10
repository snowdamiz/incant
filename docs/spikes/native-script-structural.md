# Native QuickJS structural commands

This extends the native `incant_script` host from the
[numeric-column slice](native-script-columns.md). It does not move the existing
document-backed `incant_play_legacy` gameplay or claim the complete PLAN 2.6
migration. Native execution still links only runtime/shared types and QuickJS,
without a project, journal, JSON adapter, command bus or agent dependency.

## Native API

The existing synchronous `update({chunks, dt})` callback remains compatible.
The same context now provides `commands`, with four operations:

| Operation | Arguments and behavior |
| --- | --- |
| `spawn(id, transform, parent, velocity)` | Explicit canonical uppercase ULID; ten Transform numbers; parent ULID or `null`; three Velocity numbers or `null`. |
| `despawn(id)` | Remove that entity. Children must be removed or reparented in the same batch; no cascading deletion. |
| `setParent(id, parent)` | Keep local Transform; parent is a ULID or `null` and may be spawned later in the same batch. |
| `setVelocity(id, velocity)` | Add/replace the actual component with three numbers, or remove it with `null`. |

Every argument is required. Numeric input is an Array or Float64Array of the exact
field count. Transform order is translation xyz, quaternion xyzw, scale xyz.
Binding types and a concrete example live in `sdk/ts/src/native.d.ts` and
`sdk/templates/native-structural/spawn.ts`. These focused native types are separate
from the existing document SDK; full schema-generated native SDK migration remains
open. No ignored extra spawn-component object or implicit ID generator is accepted.

Input fields are copied into owned typed Rust commands, without serialization.
Active ECS Float64Array subarrays can be passed directly: subsequent numeric writes
do not change the queued arguments. Structural value/topology validation remains
in the existing native apply path. Direct numeric writes gain no per-tick numeric
or authoring validation scan.

## Lifetime and atomicity

Each invocation owns fresh bounded queue state. Its four native closures retain
only that state, never the world, table pointers or JavaScript values. Activity
begins immediately before calling the behavior and ends immediately after return,
before detachment or diagnostic getters. The state is never reused by another
tick. Retained functions reject both between ticks and during future ticks. Drop
revokes activity and releases the native queue allocation even when JavaScript
retains a function. Command objects have null prototypes so exposure cannot call
an inherited guest setter.

Bindings parse fixed scalar fields through QuickJS property access. They do not
hold RefCell borrows or form native slices into JavaScript arrays while a getter
could reenter, throw, allocate or mutate another value. The queue limit is checked
before parsing and again at insertion, covering reentrant getters. Input errors
and overflow poison that invocation even if JavaScript catches the exception;
a prefix cannot commit silently. The queue holds at most 65,536 commands, separate
from the VM's 32 MiB heap cap and the existing 32 MiB numeric undo cap.

The success sequence is:

1. Run one synchronous callback over the actual borrowed columns and collect
   commands; no structural mutation occurs while a view exists.
2. Revoke command functions and detach every rooted external ArrayBuffer before
   diagnostics, Rust reads or error handling. Check CPU expiry, promise creation,
   pending jobs, undefined return and sticky binding failure.
3. Move only owned numeric undo values and stable-ID/layout metadata out of the
   loan, destroy all raw pointers, and end the exclusive column scope.
4. Apply the native structural batch while the host still owns `&mut NativeWorld`.
   Only on acceptance is the numeric undo discarded.

`NativeWorld::apply` rejects external data before changing ECS storage. On rejection,
the host starts a new safe column scope and compares *all* saved table/row IDs,
column lengths and Velocity presence before writing any undo value. It then
restores safe slices and refreshes derived transforms. No old pointer crosses
apply, and there is no intervening world observation. This relies on apply's
existing atomic-rejection contract; an internal layout mismatch is an invariant
assertion, not permission to restore into a different table.

Callback exceptions, CPU/VM memory failure, promises/deferred work, invalid return,
binding failure and structural rejection leave no numeric or structural prefix.
The host then fails permanently; the guest VM's private variables are not rewound.
Retained arrays/subarrays/DataViews stay detached after both accepted relocation
and rejected sync. Rust unwinding inside the loan retains its existing RAII
cleanup; process abort/native allocator failure cannot run rollback destructors.

Structural apply and any restoration run as **host sync work**, after the JS
thread-CPU deadline check. Their cost is not covered by that JS execution budget
and must be accounted for separately in the future profiler. Topology commands
may copy ID/parent metadata and check the resulting hierarchy. Component-only
queues use the existing targeted apply path. There is no per-tick scene clone.
No shipping profile, reference-phone timing or performance improvement is claimed.

## Verification

Local arm64 macOS verification passes all 373 workspace tests across 114 suites,
including 21 native host tests and all five runtime doctests. Workspace all-target
Clippy with `-D warnings`, formatting, generated conventions, generated bridge/SDK,
strict SDK TypeScript, all 25 Python tool tests and the authoring-free dependency
guard pass. Ten new behavior tests cover the four bindings, actual component
add/remove and row relocation, 2,048 script-created entities, repeated ticks,
retained functions and views, reentrant input getters, queue limits including
reentrant overflow, and copying active typed views into command arguments.

Failure cases include hierarchy cycles/missing parents, duplicate IDs, children
left after despawn, invalid spawn values/Velocity, and Infinity written directly
into a Transform before a topology command. The last case proves that validation
uses the just-written numeric values, then restores the original components and
derived hierarchy on rejection. Exception, ignored promise/async, CPU, memory,
late return and diagnostic-getter cases prove command and numeric atomicity.

Independent Astra review of parsing/reentrancy, revoke/detach sequencing, owned
undo layout checks, SDK declarations and failure tests found no actionable issue.

The checked TypeScript example was compiled with pinned SWC and run by the native
development executable in an isolated working directory with a cooked binary
scene and no project/journal. Fresh processes at 0, 1, 2, 3, 4 and 60 ticks each
ran twice with identical output. Entity count starts at two, becomes three for
ticks 1–3, and returns to two from tick 4. Spawned Velocity advances its local x;
the bulk callback advances y on rows present at callback entry. All expected
positions match within 1e-12, zero ticks restores the original scene, and the
cooked scene hash is unchanged. Reparenting and Velocity removal are additionally
asserted through typed native inspection in the focused tests. Hashes, versions
and outputs are recorded in [probe evidence](evidence/native-script-structural-2026-10-10.json).

This is source-independent **scene** execution: the development runner still
accepts CommonJS and compiles it once at startup. It is not evidence for the
authenticated, no-source-parsing shipped bytecode gate. Hosted platforms,
authenticated export loading, other components, legacy gameplay migration and
performance/device gates remain open.

Verification commands:

```sh
./tools/cargo fmt --all --check
./tools/cargo test -j2 --workspace --release --locked
./tools/cargo clippy -j2 --workspace --all-targets --locked -- -D warnings
./tools/cargo build -j2 -p incant_script --examples --release --locked
python3 tools/check_runtime_dependencies.py incant_types incant_runtime incant_script
python3 tools/generate_conventions.py --check
python3 -m unittest discover -s tools/tests -q
node tools/build_bridge.mjs --check
node tools/generate_sdk.mjs --check
node_modules/.bin/tsc -p sdk/ts/tsconfig.json
node tools/build_script.mjs sdk/templates/native-structural/spawn.ts artifacts/native-script-structural/structural.cjs
target/release/examples/native_bulk artifacts/native-script-structural/play.scene artifacts/native-script-structural/structural.cjs 3
```

Full local logs are `/tmp/incant-native-structural-workspace.log`,
`/tmp/incant-native-structural-workspace-clippy.log`,
`/tmp/incant-native-structural-build.log` and
`/tmp/incant-native-structural-tools.log`. Raw standalone outputs are in the
ignored `artifacts/native-script-structural/` directory.
