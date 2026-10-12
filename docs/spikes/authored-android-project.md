# Authored Android CPU archive route

The separate `tools/export_android.py` exporter freshly cooks public TypeScript
project sources, captures typed scene/data bindings and builds a shipping static
archive for the exact reviewed macOS ARM64 to Android ARM64 route. It validates
the selected deterministic or fast physics graph and actual target artifacts.
Desktop export retains its host-equals-target contract. Static Wasm mobile export
remains explicitly unsupported.

The production admission follows the independently accepted opt-in Android
bytecode experiment. Source review and the focused checks pass: 12 script tests,
10 cook tests across normal/probe features, 40 Python tests, three-package
all-target Clippy, formatting and diff checks. The first invalid source-map test
fixture and Python temporary-path failure remain in the lossless evidence.

The fresh authored shipping and source-deleted emulator proof is **pending**.
Archive compilation alone is not execution. The NDK API24 compiler choice is not
an API24 execution or minimum-OS claim. No physical-device, performance, signing,
store, mobile presentation/input/audio-host or phase gate is claimed.

Evidence: `evidence/authored-android-project/source-checks.json.gz` and independent
`source-review.json`.
