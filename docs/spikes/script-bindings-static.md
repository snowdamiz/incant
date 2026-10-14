# Static generated script bindings

The trusted Rust emitter now writes borrowed constant attachment/property tables
and calls one shared startup conversion. Previously it emitted a separate owned
`ScriptInstance`, `BTreeMap`, `String` and nested collection constructor for each
attachment. This reduces generated executable constructor code without changing
runtime project authority, bytecode admission or canonical binding identity.

The input is a typed borrowed Rust value, not JSON or a new persisted format.
Before allocating the owned catalog, the converter checks aggregate node/byte/
depth limits, finite numbers and strictly sorted distinct object keys. It uses the
existing property budget and then the unchanged `ScriptBindings::new` identity
and catalog validation. Numbers use their exact finite f64 bits, including
negative zero. Strings remain escaped Rust literals. No Cargo dependencies,
feature selections or lock identities changed; compiler-free players only need
the small materializer.

## Frozen source and current controls

Source `3c81bbe6763f78c10c7d13d08975f2ffeff7caa3` is based on main259
`64d536d5388349da07ce44f78c24b2ef3663e104`. All six edited paths were frozen and
asserted by the qualification drivers. The packet also reconstructs 3,881 build
input hashes from that immutable Git commit after qualification; it does not
mislabel that reconstruction as a pre-execution live workspace inventory.

Fourteen successful command receipts cover 138 library tests, 12 existing project
binding tests and six new static binding tests (156 distinct cases), strict lint,
compiler-free native and wasm compilation, normal runtime dependency isolation,
format/convention checks and the generated-source control below. The new tests
cover all value kinds, escaped text, nested arrays/objects, exact f64 bits,
canonical 10,000-row identity, duplicate/nonfinite/depth/count/aggregate-budget
refusals and a real ScriptHost's observable translation from borrowed properties.
These are deterministic Mac CI-profile executions, not shipping measurements.

A repeated emitter test writes a fresh compiled artifact plus a 10,000-row
binding module. An actual standalone Rust consumer compiles those generated files,
loads the trusted artifact and verifies every attachment ID/source/ordinal,
complex first-row properties, exact fingerprint, bytecode hash and hardening hash.
This fixture's JavaScript is deliberately minimal; the external consumer does
not execute 10,000 gameplay instances. The separate ScriptHost test exercises
real property use. Generated source is 2,341,746 bytes including the artifact
wrapper; compilation took about 2.82 seconds at opt-level 1 in this specific
standalone control. Neither number is an equal-input shipping comparison.

The first standalone compile driver omitted Cargo's host dependency search path
and failed E0463 before producing an executable. Its source, log and exit remain
retained. Adding that search path, with no generated or production source change,
produced the passing consumer. All actual child processes are reaped. Four
immediate executable copies remain local with exact hashes; the portable archive
contains their identities, logs, frozen sources and generated text, not binary
payloads.

## Evidence and limits

[Manifest](evidence/script-bindings-static/manifest.json) binds 55 raw records,
51 deduplicated payloads, four executable identities, six frozen source paths and
the complete committed input inventory. Replay without building or executing:

```sh
python3 docs/spikes/evidence/script-bindings-static/verify.py --verify-only
```

Add `--originals --local-binaries` on the producer machine to verify retained raw
files and executable copies. `--historical` permits later changes while reporting
them explicitly and still verifies every immutable source input; a later union
must separately qualify those changes. The source review is independently
accepted; final packet review and cache composition remain pending.

The original Core fast-10k shipping build and its 3,520,096-byte generated source
remain separately preserved. This packet does not claim that build completed,
that this emitter is faster on identical shipping input, a runtime performance
improvement, or a phase/release gate. Static conversion still allocates the
bounded owned catalog once at startup.
