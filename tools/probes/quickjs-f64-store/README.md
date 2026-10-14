# Diagnostic: QuickJS numeric Float64 store fast path

This is a reviewable experiment, not a production dependency patch or an accepted
optimization. The root workspace, Cargo.lock, registry cache, authored project,
ScriptHost, loan semantics, VM budget and shipping gates are unchanged. No
performance result is asserted by these files.

The exact Core stress callback performs 29,997 Float64 stores and 19,998 actual
readbacks per tick, plus sentinel stores, for 9,999 rows. `Loan::expose` exposes a
native borrowed buffer once per chunk; there is no Rust callback per row to remove.
Pinned rquickjs-sys 0.14.0's `OP_get_array_el` already tries the typed-array reader.
`OP_put_array_el` only specializes normal arrays, then calls `JS_SetPropertyValue`
for numeric Float64 stores. The retained shipping sample contains that setter
under the interpreter. Its modest sample share does not establish that removing
this overhead will close the 2 ms gate.

The patch adds one branch to `OP_put_array_el`: object class Float64Array, integer
index within the current element count, already numeric value, mutable backing
buffer. It stores the integer or float Number payload, releases the same array
reference, and performs the same stack adjustment. No user-code conversion is
possible on that branch. Every other case retains the original fallback. In
particular, coercion can detach/resize a buffer and must still precede the original
setter's bounds/immutability decision. Detachment zeros the typed-array count;
the candidate cannot store through an old detached pointer. The patch does not
change read paths, bytecode format, other numeric types or engine bindings.

## Preparation and controls

`prepare.py` copies the exact cached rquickjs-sys crate three times. It checks the
original `quickjs.c` SHA-256, retains every original file hash and license, applies
the explicit patch to the candidate, and makes a deliberate wrong-store control
from the candidate. Only quickjs.c may differ. No cached source is edited. The
wrong-store binary must fail the independent first numeric-store assertion; this
proves the new branch is actually exercised, rather than only comparing two
unreached implementations.

`prepare.mjs` compiles the unchanged, hash-pinned actual stress.ts once using the
pinned local TypeScript. Both interpreters receive those identical callback bytes.
The inherited fixture checks every actual row, finite checksum, padding value and
controller row on every one of 500 ticks, outside the optional update interval.
It also checks fresh views, changed stride, reordered rows and replaced attachment
metadata, plus three negative controls: omitted x store and NaN sentinel y/z.
There is no sample or project mutation.

`semantics.js` has 58 deterministic cases (the original 54 plus four direct numeric resize cases). It records bytes, Number observations,
errors and coercion/trap order, covering integer/float tags, signed zero, NaN,
infinities, extreme/subnormal values, normal and invalid/property indexes,
BigInt/Symbol errors, conversion hooks and getters, thrown/nonprimitive conversion,
detached and immutable buffers, resize/detach during conversion, direct numeric
stores after shrink-to-OOB/regrow on both fixed and tracking views with offsets, proxies,
subclasses/offset views, other typed arrays and assignment-expression results.
The Rust harness separately detaches an owned rquickjs ArrayBuffer and verifies
that its retained view stays unusable. This is not a substitute for Incant's
actual loan, rollback, deadline, restore or capability tests.

After offline Cargo metadata reconciles each copied lock, `check_lock.py` refuses
any dependency identity absent from the root lock except the named diagnostic
package and local rquickjs-sys copy. Baseline and candidate locks must be identical.
Use identical pinned Rust 1.99, explicit aarch64-apple-darwin, CI profile, -j2,
RAYON_NUM_THREADS=2 and a fresh, distinct target directory for EACH variant. Local path patches with
identical package identities can otherwise reuse an earlier variant artifact.
Check the build output quickjs.c against that variant, require distinct binary
hashes, and retain the wrong-store failure. Retain actual compiler/version/CC
identity, argv, process outcomes, immediate executable copies and every prepared
source hash. The CI profile is not a shipping-performance qualification.

No build or run is authorized merely by this README. Obtain the coordinated CPU
slot and source review first. Example preparation (paths intentionally explicit):

```sh
python3 tools/probes/quickjs-f64-store/prepare.py /absolute/cached/rquickjs-sys-0.14.0 /fresh/probe
/pinned/node tools/probes/quickjs-f64-store/prepare.mjs /fresh/probe/callback
./tools/cargo metadata --offline --format-version 1 --manifest-path /fresh/probe/baseline/Cargo.toml
python3 tools/probes/quickjs-f64-store/check_lock.py Cargo.lock /fresh/probe/baseline/Cargo.lock
```

Repeat lock preparation for candidate and wrong-store. Build each with the locked
explicit CI configuration, run baseline and candidate with `callback --functional`
under `tools/native_process.py` with a 120-second limit and compare the complete
58-case semantic record, three negative-control records and successful stress
coverage. Run wrong-store once and retain its required nonzero assertion failure.
No failed sample is silently retried. Any optional `--measure` invocation needs a
separate bounded run plan: it records all 500 actual update intervals, including
the first; checks remain outside those intervals. It must not be labeled host,
shipping, full Core, FPS or phase-gate evidence.

## Maintenance and security boundary

A production change here would create an interpreter fork. That entails tracking
upstream security updates, rebasing the exact patch, preserving licenses and
source fingerprints, checking all supported representations/targets, and rerunning
relevant upstream conformance plus actual hardened-host/loan tests. An inline
branch may increase dispatch code size or slow non-Float64 workloads. Passing
these bounded differential controls does not prove universal JavaScript
conformance, memory safety, profitability, or future compatibility. No production
vendoring or normal feature enablement is proposed until this diagnostic shows a
material benefit and those separate obligations are reviewed.
