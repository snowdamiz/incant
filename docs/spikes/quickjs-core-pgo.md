# Core C-PGO: functional qualification and mixed CPU outcome

The explicit, default-off experiment builds and runs Core Sample with ordinary,
instrumented and profile-use QuickJS C code. Full gameplay and audible PCM match,
but the matched CPU result does **not** justify adoption: both win-route repeats
remain above the 2 ms ScriptTick p95 target, and their measured changes have
opposite signs. Ordinary builds remain the default. No runtime scheduling,
TypeScript, SDK or project-authoring behavior is replaced.

## Trusted selection and compatibility

`tools/probes/quickjs-core-pgo.py` consumes an engine-operator request for the
existing 10,000-script Core scene and its fixed win, loss and restart recordings.
It is separate from the public exporter. Project documents cannot select tools,
C flags, profile data or diagnostic features. Physics uses the ordinary `fast`
scalar recipe from `physics_export`, without SIMD8.

`diagnostic-quickjs-generate` and `diagnostic-quickjs-use` require one closed
`INCANT_QUICKJS_PGO_DESCRIPTOR`. Feature-off builds refuse a stray descriptor;
combined modes, unsupported targets/configurations, ambient compiler overrides,
linked inputs, unknown fields and changed source/tool/profile bytes refuse.
The current diagnostic supports native arm64 macOS only. Its descriptor binds
four C translation units, fourteen headers, compiler and archiver, selected
SDK/resources, profile tool/runtime, actual ordered C recipes, mode and profile
bytes. It is trusted compilation input, not project data or authentication.

Host cooking and the final runtime have different real Cargo recipes: host
QuickJS remains O0 with ordinary copied OUT_DIR source names; target QuickJS
remains O3. Both independently validate the same descriptor and report their
actual roles. Only the target C units generate or consume a profile. Their
relative `quickjs/<unit>` names preserve static function identity across output
directories. The natural `cc` invocation spelling is preserved while its resolved
executable and ordered arguments are checked. The host is not forced to O3 and
never consumes a profile collected from a different C recipe.

The verified mode/tool/profile identity enters `INCANT_SCRIPT_BUILD_ID`;
diagnostic `default_c_flags` is false. Existing bytecode and checkpoint program
identity reject mismatched builds; cross-target default-C-flags admission stays
closed. Adding build support also changes the conservative vendor identity for
ordinary builds, requiring a fresh cook. Historical cooked programs are not
relabeled compatible. Packaged raw scene/assets stay byte-identical across the
three modes; compiled program/header identities embedded in each executable are
separate and mode-specific.

Only generation links the pinned Clang profile runtime and retains final code
with `-C link-dead-code=yes`. It is nonshipping, retains parser APIs and has no
source-free or performance claim. The audit verifies the exact nested archive
payload in Cargo's precise runtime rlib and its unique owned force-load operand.
Instrumentation is proven before stripping, then tied to the packaged image by
UUID and identical code, initializer and profile sections. Optional empty Clang
bits/vns/vtab declarations are distinguished from required real counters, data,
names, code and initializer bytes. Ordinary/use retain their normal final-link,
source-free dependency/symbol and shipping codegen guards; neither contains the
profile-generation runtime.

## Actual functional producer

Native qualification ends at `4ed9ae187c94c5c9e195d9c73a0726e0940afce9`.
The successful coherent producer and timing source is
`af775fd1c43318ddaa38ce61628e5f4e7b8b4275`; subsequent changes between those
identities are Python audit/test corrections, with native source bytes unchanged.
The source branch starts at main393 `5866170f`; main405 is a later integration
baseline, not the producer of these binaries.

The successful grant is packet008, retained under `core-shipping-007`. Actual
exec41416 exits zero after 793.964 seconds and 146 managed calls. Independent
closeout confirms all 152 observed identities/groups absent. One fresh
`FreshSession` preserves immutable origins for shared unchanged units and builds
a new scene-bound player for each mode. All four sys/script cooker/runtime
receipts and the actual feature graph pass. Generation trains only on win2156;
its sole 277,808-byte raw profile feeds a strict, same-toolchain merge using
`--failure-mode=any -j 2`, without sparse mode. The indexed profile is 319,928 bytes,
SHA-256 `9a6439175ee910f075c14f534322b31aac13b49a6ab5688cbacbad61ac80000b`.
Discarded compiler/canary and held-out data are excluded.

Copied authoring, build storage, descriptors and discarded profiles are removed
before the six ordinary/use validation children. Every route's complete report
and full 48 kHz stereo f32 PCM match, and training win also matches ordinary win:

| Route | Host steps | Final world tick | Result |
|---|---:|---:|---|
| Win | 2,156 | 2,156 | Exact gameplay and whole PCM |
| Held-out loss | 900 | 900 | Exact gameplay and whole PCM |
| Loss/restart/win | 3,056 | 2,156 | Exact gameplay/PCM; generation 2, restart at host step 900 |

`frames` in these reports counts PCM sample frames, not gameplay ticks. Raw
profile data is needed for compilation, not standalone use-player startup.
The manifest publishes only after parity, input rechecks and player rehashes.
The players are 12,985,296 bytes ordinary, 13,988,912 bytes generation and
12,788,512 bytes use; exact identities are in the portable manifest and receipts.

Six earlier actual producer failures remain failed: final-link wrapper parsing,
canonical Cargo dependency naming, the assumed host C recipe, a relative script
admission path, nested profile-runtime archive shape, and post-strip private
symbol inspection. Native prerequisite failures and their resource charges are
also retained. These were not silently adopted as successful builds.

## Matched CPU timing

The same retained ordinary/use players run once in the frozen order
`O0,U0,U1,O1,U2,O2,O3,U3`, with two win and two held-out loss runs per mode.
All native build/GPU work is held during timing. The Mac is Mac17,9, Apple M5 Pro,
arm64, macOS 27.0.1 build 26A434, AC Power; the low-power-mode field is absent,
not inferred disabled. Nine saved metadata sets match.

The maintained heap observer and profile instrumentation are identical. Every
frame is included, including startup/warmup; no quantiles are pooled. The actual
exec8374 exits zero in 36.018 seconds. All 80 managed calls confirm cleanup;
independent fresh checks find the wrapper and 80 leader PID/groups absent. All
12,224 ticks and 78,233,600 bytes of PCM match the earlier functional reports.

| Pair/route | Ordinary ScriptTick p95 ms | Use ScriptTick p95 ms | Use/ordinary ratio |
|---|---:|---:|---:|
| 0, win | 2.270001–2.280000 | 2.170001–2.180000 | 0.951755–0.960352 |
| 1, loss | 1.900001–1.910000 | 1.830001–1.840000 | 0.958116–0.968421 |
| 2, win | 2.230001–2.240000 | 2.410001–2.420000 | 1.075893–1.085201 |
| 3, loss | 1.920001–1.930000 | 1.880001–1.890000 | 0.974094–0.984374 |

These are 10 µs histogram quantization bounds, not statistical confidence
intervals. Nearest rank is `ceil(.95*N)`; ratio bounds are
`[use.lower/ordinary.upper, use.upper/ordinary.lower]`. All required ScriptTick and
PlayerFrame rows have complete sampling with zero missing, invalid or overflow.
Other raw rows keep their explicit invalid/missing states. Per-source wall and
thread CPU observations are means, not source p95 values or a subtraction from
another phase's p95.

PlayerFrame p95 for win is 3.130001–3.140000 → 2.950001–2.960000 ms in pair0,
but 3.080001–3.090000 → 3.530001–3.540000 ms in pair2. The use win repeat ratio
U2/U0 is 1.105505–1.115207 for ScriptTick, while O2/O0 is
0.978071–0.986784. The held-out loss result is modestly lower for use, with both
modes already below 2 ms. These observations establish neither a repeatable win
improvement nor why the repeats differ. All four win arms fail the 2 ms target.

The first timing wrapper, exec29495, remains a prelaunch failure with zero native
children or samples. Its wrapper PID was not observed. A conservative 5-second
unmeasured debit is retained; the corrected successor records identity before
initialization and tests the actual Guard constructor. Across prior failures,
prerequisites, producer and timing, 3,952.572 of the original 5,400 seconds are
charged. Sampled resource maxima and all revised bounds remain in the records;
deleting the final build target earns no accounting reset.

## Portable replay and remaining work

[Portable evidence](evidence/quickjs-core-pgo/manifest.json) contains 373 inert
text records in a 1,347,898-byte gzip archive. It preserves positive and negative
producer/qualification records, actual role/recipe/link/image audits, all eight
raw timing reports, selected raw streams and process closeouts. Native players,
package data, full PCM and profile bytes remain external with exact identities.
Replay checks data only, never executes archived source or a player:

```sh
python3 -B docs/spikes/evidence/quickjs-core-pgo/verify.py
python3 -B docs/spikes/evidence/quickjs-core-pgo/test_verify.py
python3 -B docs/spikes/evidence/quickjs-core-pgo/verify.py --originals --artifacts --git-source af775fd1c43318ddaa38ce61628e5f4e7b8b4275 --repo .
```

Forty-two current Python source controls and eighteen portable refusal/replay
controls pass. Native tests, actual descriptor-owner executions and strict lints
remain bound to their recorded prerequisite sources; the final functional builds
exercise actual generation/use in both roles. No new native run is needed to
finalize this report. This is not complete SDK/other-target qualification, a
rendering/window/FPS result or phase approval.

The next useful trigger is evidence of a specific dominant residual QuickJS or
SDK cost that earlier loan/navigation experiments did not cover, with a credible
bound toward 2 ms. A bounded CPU-stack investigation of the actual current hot
script could distinguish interpreter property/number work from host conversion;
function counts and these unmatched p95 differences do not establish that cause.
Do not repeat the unchanged eight arms or infer a new optimization from generic
batching advice: the engine already uses native ECS and typed bulk columns.
