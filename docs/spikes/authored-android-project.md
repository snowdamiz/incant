# Authored Android CPU archive route

`tools/export_android.py` freshly cooks public TypeScript project sources,
captures typed scene/data bindings and builds a shipping static archive for the
reviewed macOS ARM64 to Android ARM64 route. Source admission follows the actual,
independently accepted [candidate bytecode experiment](android-bytecode-candidate.md).
The loader checks the exact QuickJS bytecode28 ABI, numeric configuration,
producer/destination identities, source/binding hashes and compiler identity;
existing cached artifacts are never relabeled. Desktop export keeps its
host-equals-target rule. Static Wasm mobile export remains explicitly unsupported.

The exporter selects the project's deterministic or fast physics family with
default features off. It audits the scoped normal dependency graph and all five
actual target solver artifacts, then verifies opt3/fat-LTO/one-codegen-unit/
panic-abort code generation. It emits a create-only runtime archive, public C
header and cooked data package, with DWARF and source maps kept separately. The
normal runtime graph excludes document, command-bus, cook and compiler dependencies.
Unused compiler definitions in the unlinked archive are recorded honestly;
`--gc-sections` and a separate final executable audit are required.

```sh
python3 tools/export_android.py /absolute/project/game.json /new/android-archive \
  --target aarch64-linux-android \
  --ndk /absolute/sdk/ndk/28.2.13676358 \
  --evidence /new/android-build-evidence
```

This bounded exporter admits the reviewed NDK28.2.13676358 macOS toolchain and
clang19.0.1. Tool identities follow trusted symlinks only inside its canonical
installation; execution preserves each original invocation name. The strict
project/package symlink refusal is unchanged. The API24 compiler selection is
recorded separately from the actual Android17/API37 execution evidence.

## Retained execution

The immutable implementation is `06d534cb`, main176 source composition
`074bd373`, public probe `ad5ececb` and trusted tool-identity correction
`8c6bc7c3`. Independent reviews accepted each production/probe source freeze.
Independent final execution review accepts the complete retained packet.

Both project modes pass public command authoring of three real TypeScript files,
shared imports, distinct per-instance properties, captured WAV audio and the
physics setting. Four Undo/Redo transactions and journal reopen recover the exact
project. Three matching native development processes per mode pass 12 ticks,
36 per-field IEEE/BigInt/Unicode log records, 72 vector checks and 76,800 bytes of
nonzero PCM per run. The independent expectation follows the engine's prior
committed velocity schedule; held/released/repeated inputs are distinct controls.

Fresh ordinary Android shipping exports then pass three separate C caller
processes per mode on a disposable ARM Android17/API37 emulator. Every numeric
record, observation and PCM byte matches the corresponding native baseline after
private project/source/cache/build storage and newly generated source-map removal.
The final callers contain `JS_ReadObject`/`JS_EvalFunction` and the C ABI, without
source parser or DWARF sections. Each mode passes seven exact boundary refusals:
create-only no-clobber, late script failure, unknown field, wrong request version,
zero ticks, relative package path and corrupted cooked data. These check actual
reported errors/status and output absence or preservation, not any nonzero exit.
Both owned emulator processes exited gracefully and their AVDs were deleted.

| Mode | Runtime archive | Final C caller | Fresh processes | Refusals |
| --- | ---: | ---: | ---: | ---: |
| Deterministic | 23,857,828 B | 12,860,016 B | 3 | 7 |
| Fast | 23,849,196 B | 12,853,928 B | 3 | 7 |

The final producer audit rehashed 13,405 unchanged frozen Git inputs, 484 raw
caller command streams, both archives/callers and 18 runtime package files.
Each caller proof contains 119 execution command receipts plus AVD create/delete.
Both actual five-solver target audits pass. The fixture observes typed velocity
motion; it does not establish collision, numerical adoption or performance gates.

Focused validation also passes 12 script cases, ten cook cases across default and
candidate features, 50 current Python cases, three-package all-target Clippy,
formatting and diff checks. Both matching headless binaries were rebuilt on the
composed source. The initial invalid source-map test, temporary-path test failure
and first shipping attempt remain retained. Shipping001 compiled successfully but
publication refused a symlink while hashing the trusted NDK tools; it published
nothing and launched no caller. Its private unpublished archive was removed by
the exporter's scoped cleanup, requiring the fresh successful shipping002 builds.

The lossless [manifest](evidence/authored-android-project/manifest.json) binds
raw command outputs, receipts, copied source reviews, per-tick observations and
PCM. The final review rehashed all 869 records, 335 payloads, eight binaries, 14 inputs,
484 raw command streams and every positive/refusal result. It verified all 13,405
frozen inputs with 39 historical Git fallbacks after branch switching.
Large actual native binaries/archives remain at the durable external paths
with exact hashes and sizes; the manifest distinguishes them from repository
payloads. Source reviews and the accepted StageA review are retained alongside it.

This is an unsigned CPU archive/C-ABI route. It does not deliver an Android
activity/JNI host, Gradle/APK/AAB, mobile rendering/touch/device-audio integration,
minimum-OS execution coverage, physical-phone results, signing, store readiness,
performance or a phase gate. The emulator is functional evidence only.
