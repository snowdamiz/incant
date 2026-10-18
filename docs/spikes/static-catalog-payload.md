# Checked static catalog payload selection

Checkpoint56's Linux, macOS and Windows Wasm conformance jobs rejected the player
under strict Clippy:
the non-authored build emits an empty `wasm-instances.bin`, so indexing the
statically known `include_bytes!` array at byte32 triggered
`clippy::out_of_bounds_indexing`. The preceding runtime empty check did not
suppress that compile-time diagnostic. All three original failed lint logs are retained;
checkpoint56 is not reclassified as passing.

Source `d4f4e1582007385a55d8897e7eba53eef08de5e7` replaces the direct indexing with
a checked split in the static-module catalog decoder. Empty bytes still mean no
instance catalog. Nonempty truncated headers and malformed payloads still fail;
the scene digest and validated frame reach the same existing catalog parser.
Linked-module selection, authored override refusal and feature selection are
unchanged. No lint suppression or platform exclusion was added.

The scoped Mac qualification passed both new catalog controls and strict
all-target `incant_game_player` Clippy with `static-conformance`, pinned Rust1.99,
locked/offline dependencies, `ci` profile and two workers. The first control
checks the actual generated zero-byte catalog as well as lengths1 through32;
the second checks an exact scene/row round trip, trailing-byte refusal and
corrupted payload refusal. The selected build-script output independently
confirms the generated file is empty. Tests and lint both returned0; actual
supervisor63629 returned0 after94.938 seconds. No retry occurred.

The tested executable was copied outside the target with SHA256
`25ff1674b5e3ab371f688596c8e7f8be9e517efe213d3d33c0a3bfd2b0f027a0`.
Fresh closeout verifies the wrapper and all92 observed child PID/group identities
are absent, and79 protected artifact identities remain intact. Shared target
allocation and retained peak are10,681,225,216 bytes under its existing12GiB cap;
prior resource charges remain included. No GPU, window, capture or audio device
was used.

The [portable manifest](evidence/static-catalog-payload.json) binds26 UTF-8
records in its gzip archive: the original three-platform failures, complete source patch,
source/tool/bundle identities, exact plan/runner/grant, raw Cargo/test/lint logs,
generated-file identity, actual terminal and fresh process/target closeout.
Decompression must match the manifest's byte count and SHA256; every record's
UTF-8 bytes must then match its declared count and SHA256. Original absolute
paths are provenance references, not required replay inputs. Executables and
the previously qualified46-file guest bundle remain external, hash-bound
artifacts and are not embedded in this packet.

This is a focused local feature-path regression proof. It does not rerun the
full Wasmtime/static suite or establish Linux/Windows success. Hosted checkpoint
confirmation remains required.
