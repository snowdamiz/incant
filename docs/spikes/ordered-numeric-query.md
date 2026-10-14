# Ordered native numeric queries

This private candidate starts at main279
`e2207d9b897eb770aa0486a724879eb95305ff9c`. It replaces repeated per-entity
`QueryState::get/get_mut` in hierarchy preparation and numeric publication with
one scoped ordered iterator per pass. It has correctness evidence only. No new
shipping run, benchmark, gain, 2 ms gate or 5% regression acceptance is claimed.

The retained main278 maintained-mode shipping profiles report mean ScriptTick
1.459–1.525 ms, with 0.345–0.355 ms outside the source callbacks. Those are means;
independent percentiles are not subtracted. The earlier real-ScriptHost CI probe
measured numeric validation at 253 microseconds and cleanup at 76 microseconds,
versus 14 microseconds for undo capture. That probe omitted the full Core game.
It identifies a region, not the cost of its individual query operations. The
failed/mixed matrix-basis, authored-loop and Float64Array-store experiments and
the main278 Loan-capacity result remain unchanged.

## Change and invariants

Pinned Bevy 0.19.1 initializes data/filter fetch state in each `get_inner` call.
Its ordered iterator initializes that state once per pass while still checking
the live entity generation, matched archetype, table and filter. Compiler
optimization may remove some existing setup; the saving remains unmeasured.

`iter_many` skips nonmatching entities. Both changed loops therefore return and
check the exact expected `Entity` before using its hierarchy index, check scratch
length, and require final exhaustion. A missing mandatory component cannot shift
the remaining rows onto another entity's scratch. Mutable results are consumed
one at a time using `fetch_next`.

Parent-first traversal, first error, scalar/UI/physics checks, quaternion
normalization, matrix multiplication and signed-zero arithmetic are retained.
Preparation still writes only scratch; the numeric scope publishes at its
existing cleanup boundary. The query is local to that call. No pointer, query
state, matrix cache or allocation is retained across world mutation. Foreign
view revocation, undo storage, rollback and guest/source execution are unchanged.
The preparation function also serves ordinary integration and solved-physics
branches, so their existing controls were included.

Three new tests observe:

- Parent-before-child refusal despite the opposite ECS table order, followed by
  complete restoration and exact committed values.
- Bit-exact agreement with the original scalar `refresh_globals` publication
  after reparenting, a Velocity component/table move, despawn and stable-ID reuse.
- Refusal of deliberately missing private mandatory components without assigning
  a skipped row's scratch to its descendant.

## Qualification

Frozen generation001 contains both changed Rust files and 3,922 build inputs.
The native target was `/private/tmp/incant-agent-font-target`, Rust 1.99.0,
explicit `aarch64-apple-darwin`, locked/offline dependencies, CI profile, two jobs
and Rayon 2. Each mode selected `profile,native-checkpoints`; script tests also
selected `compiler`. Exact argv, environment, logs and immediate executable
identities are in the [CPU receipt](evidence/ordered-numeric-query-main279.json).

For **each** of deterministic and fast-SIMD8:

- Runtime library: 48 passed.
- Script library: 162 passed; one existing phase diagnostic remained ignored.
- Runtime integration targets `numeric_boundaries`, `columns`, `physics`,
  `physics_2d`, `ui`, `animation`: 55 passed.
- Strict all-target Clippy for runtime and script passed.

Workspace formatting and whitespace checks passed. Thus each mode has 265
passing tests; this is not a claim of all workspace test execution. Sixteen
executable copies were retained immediately after their commands. Supervisors
40821 and 91730 exited zero and reaped their managed descendants; the native
target/slot was released.

Two earlier launcher failures are preserved. Attempt001 could not find `cargo`
in the inherited PATH. Attempt002 reached Cargo but could not start `rustc -vV`.
Neither compiled the source. Attempt003 explicitly bound the Rust 1.99 toolchain
bin directory and absolute `RUSTC`/`RUSTDOC`, then passed the library/lint chain;
attempt004 passed the integration targets without source changes.

External originals, source copies and command records remain under
`~/.codex/incant-evidence/ordered-numeric-query-main279-001`. This receipt binds
their bytes; it does not embed the native binaries. Root owns independent review,
integration and any later coordinated fixed-protocol shipping comparison. The
authored workload, whole-tick accounting and acceptance thresholds remain fixed.
