# Generated native Rust module SDK

This Phase 1 slice provides the public, allocation-free `incant-module` Rust guest
API over the existing native numeric/entity, structural, input and flipbook
capabilities. It does not add a second world mutation path or an authoring tick
path. The usable sample and build command are in [the SDK guide](../../sdk/rust/README.md).
No complete gameplay SDK, reference-phone, browser, export or phase gate is claimed.

## One registry and two actual hosts

[`sdk/abi/native-v1.json`](../../sdk/abi/native-v1.json) owns the versioned import
names/signatures, C dispatch opcodes, byte records/offsets, endian conventions,
enum tags, input section order and transfer bounds. The standard-library Python
generator emits a lean `no_std` shared ABI crate, private guest imports, the real
Wasmtime linker adapters, static Rust/C dispatch, C namespace contexts and input
tag mapping. Generated-file equality is required by the existing three-desktop
Wasm workflow. JSON remains build input only; no shipped runtime parses it.

The host writer and guest parsers use the same generated field-by-field endian
codecs. They expose initialized numeric bytes, never native pointers or padding.
Native Transform/Velocity sizes and field offsets are compile-time checked
against these wire records. Old namespace versions, signatures, opcodes and bytes
are unchanged; prior WAT modules and exact input-byte expectations remain tested.
Unknown namespaces/signatures and snapshot magic/stride versions fail closed.
The cook's compiler graph stays outside normal runtime dependencies.

The safe SDK layer uses callback-bound Tick/Chunk values and initialized caller
scratch. Columns are copied, with full-span capacity checks. Input and playback
views borrow copied scratch and support allocation-free bulk iteration. Numeric
read-after-write sees the staged copied columns; playback read-after-control sees
the prior committed snapshot until the callback succeeds. Native structural and
numeric validation/publication remains in the original shared transaction code.

Returning an SDK callback error traps. Ignoring an error returned by an active
SDK operation also traps at callback completion, so earlier numeric/control/
structural prefixes cannot commit. Pure snapshot parsing only validates copied
caller data; its error must be propagated to reject the callback. Invalid host
imports trap immediately, including old epoch reuse. A failed module is terminal;
guest state and already accepted simulation/device audio are not rolled back.

## Trusted build integration

`tools/rust_guest.py` invokes pinned rustc directly on exactly two fixed library
roots and the trusted guest source. It runs no Cargo/project build script or
proc macro. Bounded reusable library outputs are identified by SDK source,
compiler and codegen hashes. This is a repository fixture/build helper, not an
untrusted Rust compilation sandbox or a runtime source loader.

The existing bulk, composed-game and flipbook Rust fixtures now use the public
SDK. Existing numerical, input, audio, source-refusal and flipbook assertions are
preserved. The composed shipping proof uses the migrated SDK guest through the
normal static-module/native game path. The standalone public sample is additionally
executed by both real Wasmtime and wasm2c hosts in the conformance suite.

The sample exercises numeric rows, stable IDs, input actions, spawn/parent/
velocity/despawn, and bulk playback readback with queued pause/seek/restart.
Adversarial samples cover a callback error, an ignored undersized buffer error,
malformed input parsing and an expired epoch after one successful update. Each
has preceding numeric, structural and playback operations; the rejected callback
publishes none of them and the host remains failed. Raw controls also exercise
unaligned byte copies and overflowing guest ranges. SDK doctests enforce that a
chunk cannot escape its callback and borrowed snapshot scratch cannot be mutated.

## Local verification

The local host is aarch64-apple-darwin, Rust 1.99.0, pinned Wasmtime 49.0.2 and
WABT 1.0.42. No third-party dependency/version changed; only local ABI/SDK package
edges were added. Windows/Linux executions of the new SDK cases remain pending
hosted integration. Existing compiler/dependency exclusions and all prior Wasm
fuel/trap/refusal controls remain enabled.

- Final combined helper: 167 tests/doctests, all-target Clippy, every registered
  import accepted with its exact signature by the real development linker/static
  cook and rejected with altered namespace/signature, plus three fresh development
  processes. All original 13 Wasmtime conformance cases remain.
- Full workspace: 692 ordinary tests/doctests and all-target Clippy with the
  existing optional scene-audio feature in the ci profile. 71 ignored GPU cases
  were not executed for this nonvisual SDK slice.
- 93 Python tools tests, generated ABI equality, conventions, workflow actionlint,
  default wasm32 SDK/ABI/development-package check and authoring-free normal graph
  checks pass. Combined/fresh helper traversals separately enforce compiler/JIT
  exclusions from shipped roots even with development features resolved.
- Public CLI compiles the real sample to wasm32. Real InputRuntime wire tests
  also feed the public parser, including all gesture kinds, gamepad/raw/named
  values, maximum accepted action names and 1,823 touch changes caused by focus
  cancellation. Malformed UTF-8/tags/counts/nonfinite values and truncated records
  are refused; unaligned scratch is supported without casts.

The [immutable evidence](evidence/native-rust-sdk.json) records all 51 changed
source/build-input hashes, check commands/log hashes and the complete fresh
shipping result. The final both-host rerun uses explicitly formatted standalone
guests and stronger truncation tests; these are the only changes after the
full-workspace check (no runtime implementation changed).

The fresh composed player passed eight replay/physics/module/audio executions
and 26 refusal controls. It is 5,495,136 bytes, SHA256
`77bd0003df2aed205885aaa948c20d816f05572217ed79fa526a71d87d6ed50c`,
with matched arm64 dSYM UUID `3e072be7-1b6d-3646-924a-ba8255f5a76c`.
The result JSON SHA256 is
`7e4826691a9d2ca9d344439c83bfc247cf4ad697612300023e3f9c1f1dab54f7`.
Fresh compile logs verify optimized fat-LTO/one-codegen-unit/panic-abort shipping
and normal compiler/JIT/source-compiler exclusions. Source, Wasm, generated C,
translator, guest SDK rlibs and fresh target storage are deleted before copied
player execution; repository originals and runtime PCM/recording files remain.
No matched reference-phone performance or binary-size improvement is inferred.

Independent root review verified all 51 final source hashes and the exact shipping
result, then accepted the parser/adapter/C mappings, bounds, callback lifetimes,
trap latch and trusted build boundary without blocking findings. Hosted integration
and broader SDK/device gates remain pending.
