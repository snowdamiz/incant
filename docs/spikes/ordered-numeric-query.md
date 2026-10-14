# Ordered native numeric queries

This private candidate starts at main279
`e2207d9b897eb770aa0486a724879eb95305ff9c`. It replaces repeated per-entity
`QueryState::get/get_mut` in hierarchy preparation and numeric publication with
one scoped ordered iterator per pass. Correctness checks passed, but the later
fixed shipping comparison did not establish a performance improvement. Keep
`b0b2a583a9d0f27691fa614e3a10a8160ea04434` as a rejected experiment; do not
integrate its production change on performance grounds. No phase gate passed.

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
optimization may remove some existing setup. The shipping result below does not
establish a saving from the replacement.

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


## Shipping result: do not adopt

Root executed one fixed sequence, `B0,C0,C1,B1,B2,C2`, on the same Mac17,9
Apple M5 Pro (15 physical CPUs, 48 GiB, arm64, macOS 27.0.1 build 26A434,
AC power). The predeclared pairs were C0/B0, C1/B1, C2/B2. Each process ran the
same 2,156-tick Core win route with 10,000 scripted entities and two source
groups. All runs used profile v3 maintained-heap observation, included the full
ScriptTick and every tick, and had no warmup exclusions, retries, subtraction
or quantile averaging. The supervised six-process sequence completed in about
32.4 seconds; its process session 85300 exited zero and all owned processes
were reaped. This duration is orchestration context, not a throughput metric.

Actual histogram-derived p95 intervals, in milliseconds:

| Run | Player frame p95 | ScriptTick p95 | Peak requested Rust bytes |
| --- | --- | --- | ---: |
| B0 | 2.880001–2.890000 | 2.090001–2.100000 | 76,881,212 |
| C0 | 2.970001–2.980000 | 2.070001–2.080000 | 76,881,342 |
| C1 | 2.930001–2.940000 | 2.110001–2.120000 | 76,881,342 |
| B1 | 2.880001–2.890000 | 2.080001–2.090000 | 76,881,212 |
| B2 | 2.890001–2.900000 | 2.080001–2.090000 | 76,881,244 |
| C2 | 2.900001–2.910000 | 2.100001–2.110000 | 76,881,342 |

All three pairs pass the unchanged interval-based 5% frame, script and
requested-memory nonregression checks. All three candidates fail the unchanged
2 ms ScriptTick gate. Frame p95 is higher in every pair; ScriptTick is lower in
one pair and higher in two. Requested peak memory is 98–130 bytes higher. This
single fixed sequence does not establish a meaningful benefit or statistical
significance. Nonregression alone does not justify adopting the extra traversal
logic.

The candidate player SHA256 is
`26aabc30ebd4f68573a4006b4519cb4523cfe088d6632d2408d25568179559ae`;
the retained main278 baseline is
`21c23c9f8aae75a4cccb506af02925ceded760bf75b8449e25fcc17df8f02a02`.
The fresh candidate used the unchanged public exporter: Rust 1.99.0, actual
`aarch64-apple-darwin` shipping codegen, fat LTO, abort, one codegen unit, fast4
physics and the same `project,render-input,profile` graph. **Neither shipping
arm enabled native-checkpoints.** The earlier request's checkpoint wording was
corrected to match the actual baseline; the separate CI correctness suites did
enable that feature.

The full runtime/tool/build Git closure differs from main278 only in the
reviewed `world.rs` change and its test file. The 74 authored inputs, all cooked
runtime data, TypeScript compiler result, source map, bytecode and ABI are exact
matches. Plain, detailed and maintained functional controls each reproduced
the full historical logical report and exact PCM before timing. Every timed
run repeated those checks. The raw timing histograms, mode/coverage, machine,
source groups, command order and interval verdicts were independently replayed.

The first export attempt failed because the explicit host PATH omitted Node.
It published no package and ran no player. Its original logs/outcome remain in
the packet. A separate attempt bound the installed Node v24.21.0 identity and
passed without source changes. The exporter removed its private target before
its built-in startup check. The old baseline and its receipts were not changed.

The focused [portable manifest](evidence/ordered-numeric-query-shipping/manifest.json)
binds 163 records and 136 deduplicated payloads, including every timed raw report,
command, original setup failure and the frozen protocol. Run:

```sh
python3 docs/spikes/evidence/ordered-numeric-query-shipping/verify.py
# When retained originals and the repository's immutable Git objects are available:
python3 docs/spikes/evidence/ordered-numeric-query-shipping/verify.py --originals --binaries --git-source
```

Both commands replay evidence only. Native executables, cooked-data payloads and PCM
remain in the retained external directories; the portable packet verifies their
correspondence hashes, and the optional flags rehash the actual files. The Git
option verifies all 3,944 frozen build-source rows against the immutable source
commit. The independent full replay passed. External originals and release
receipts are in `~/.codex/incant-evidence/ordered-query-shipping-main279-001`.
The earlier 2 ms failures and all other open gates remain unchanged.
