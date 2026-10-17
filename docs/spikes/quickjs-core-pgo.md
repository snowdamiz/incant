# Core C-PGO experiment — source preparation

The explicit diagnostic uses an ordinary feature-off baseline, a nonshipping
instrumented training player, and a profile-use candidate from one frozen source.
No Core build, native qualification, measurement or adoption is claimed here.
The preceding [C feasibility](quickjs-c-pgo-feasibility.md) result does not establish
engine compatibility or a speedup. The existing 2 ms p95 requirement stays open.

`tools/probes/quickjs-core-pgo.py` accepts an engine-operator request for the
existing 10,000-script Core scene and its win, loss and restart recordings. It is
separate from the public exporter; project documents cannot select C flags,
tools, profile data or diagnostic features. Physics selection uses the ordinary
`fast` scalar recipe from `physics_export`, with no SIMD8 feature. The earlier
external proposal's SIMD8 wording is historical and is not implemented.

The two opt-in Cargo features, `diagnostic-quickjs-generate` and
`diagnostic-quickjs-use`, require one closed `INCANT_QUICKJS_PGO_DESCRIPTOR`.
Without either feature, a stray descriptor refuses. Both features together,
foreign host/target, unsupported C configurations, ambient compiler overrides,
linked inputs, unknown fields, or changed source/tool/profile bytes refuse.
The descriptor binds all four original C units and fourteen headers, compiler,
archiver, SDK/resource inputs, profile tool/runtime, ordered ordinary target C
recipe, mode, and actual profile bytes. It accepts native arm64 macOS only.

Both the host cooker and target runtime validate that descriptor independently.
Their real Cargo/cc commands must use the same C recipe with only the exact
known contiguous PGO additions; relative `quickjs/<unit>` paths preserve static
profile names across distinct Cargo output directories. The ordinary copied
OUT_DIR recipe is unchanged. Diagnostic host QuickJS explicitly uses the ordinary
target's O3/debug C recipe, rather than assuming its usual host recipe matches.
Cold receipts require all four sys/script host/runtime roles and matching cooked
build identity. Missing or stale profile diagnostics remain errors.

The verified descriptor digest enters `INCANT_SCRIPT_BUILD_ID`; diagnostic
`default_c_flags` is false. Existing bytecode admission and checkpoint program
identity continue to refuse mismatched builds, and cross-target default-C-flags
admission stays closed. Adding the build support changes the conservative vendor
identity even when feature-off; historical cooked programs remain historical.

Only generation links the verified Clang profile runtime and uses final-binary
`-C link-dead-code=yes`. Its parser/runtime symbols are deliberately retained, so
it has no source-free or shipping claim. Baseline/use keep existing source-free
graph, symbols, Rust shipping codegen and final-link checks. Actual final link
commands, profile sections and symbols are retained in all modes.

One disposable `FreshSession` keeps immutable fresh origins for unchanged target
units and packages each player before retiring its root units. The instrumented
win replay trains before deletion, under a separate explicit raw-output path.
Only its single inventoried raw profile is merged with the same profdata tool,
`--failure-mode=any -j 2`, without sparse mode. Compiler/discard and held-out data
are excluded. Use compilation must pass the strict guards in both Cargo roles.

After packaging all three players, copied authoring, build storage, descriptors
and discarded compiler profiles are removed before baseline/use validation.
Standalone baseline/use win, loss and restart reports and whole PCM must match
exactly; training win must also match baseline win. The manifest is written only
after those checks, immutable input rechecks and player rehashes. Failed partials
remain incomplete. A managed resource/ownership supervisor must bind the actual
tools, inputs and finite storage/time bounds before any native execution.

Portable source controls cover ordered vectors, profile names, both dependency
roles, identity, training-only linkage, input drift and deletion-claim scope.
Rust admission/checkpoint controls are prepared but have not run. Actual engine
builds and functional parity precede any separately reviewed shipping timing
comparison. Training time never counts as performance evidence.
