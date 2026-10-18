# Registered candidate installation

Source implementation, not native qualification. This child composes the reviewed
compiled-Wasm transaction foundation with registered source tools. The ordinary
compiler host remains experimental macOS-only. The new local/agent controls are
authored; none have been compiled or executed for this source yet. The actual
Local compiler control is ignored by default and must be explicitly scheduled.
No shipping, activation, gameplay, device, performance or phase claim follows.

`module_install_registered` accepts only the registered module ID, completed
snapshot/candidate IDs, observed document revision, source/destination versions,
and description. `module_read` now also returns `compiled_destination`; a refused
destination is reported separately without hiding readable Rust source. There is
no arbitrary candidate/project path, compiler, shell or raw-byte input. Existing
read/write/inspect/build and legacy tools retain their interfaces. Generated tool
schemas remain an integration follow-up, not a claim from source formatting.

Before approval, the adapter captures the bus session, registration, revision,
source and destination CAS. After approval it rechecks authority. The service
resolves only a complete registered result in its current generation, validates
the worker receipt and pins/copies the bounded private candidate (1 MiB maximum).
The ordinary importer regenerates `CapturedModule` validation and compares raw
and prepared hashes with that receipt. The source snapshot is limited to 32 Rust
files, 64 KiB each, 256 KiB total, 128 entries and eight package levels.

An opaque, nonserialized `Installation` owns both the service lease and ordinary
import preparation. It has no accessor returning detached candidate bytes or the
prepared import. Backend package recapture runs outside bus and service locks.
A read-only `RustPackageSnapshot` holds the complete confined directory chain,
source nodes, membership and contents. At publication the lock order is bus then
service; the held package, source epoch, candidate and authority are checked
again before invoking the command transaction. Close/bind cannot cross this
publication boundary. Cancellation before the boundary refuses; cancellation
once journal mutation begins cannot conceal a completed/recovered transaction.

The final bounded package check catches direct sibling changes after preparation,
including changes while waiting for the bus. It is not an atomic filesystem
snapshot against arbitrary external writers modifying files concurrently with
or after the last check. Retained handles pin in-process identities. Serialized
IDs alone do not prevent inode/file-ID reuse after process exit. The existing
same-path reopen and moved-project refusal boundaries remain unchanged.

Registered Local inspection uses this native read-only package guard so an
installed entry's exact `.wasm` sibling does not prevent rebuilding. Only that
one derived filename is admitted as a bounded, single-link, no-follow output;
it is never copied, hashed into the Rust source digest or passed to the compiler.
Destination CAS owns its bytes/version. Other binaries, aliases, links and
manifests still refuse. Use separate package directories for separately compiled
entries: another entry's `.wasm` is not implicitly trusted. Local tool/config/SDK
identity and root/storage overlap checks remain. Legacy Python capture stays
strict; actual build still materializes only Rust snapshot files and runs its
unchanged strict capture on that private source tree.

Publication is one ordinary `incant_cmd` asset/binary transaction. The journal
owns exact bounded binary nodes and source/destination hashes; actor provenance,
Undo/Redo, crash recovery and first-package ownership compose through that path.
Worker/source-digest receipt details are returned as installation result data;
they are not fabricated native evidence. No scene selects or activates the asset.
After success, removing the service's private candidate cannot break journal
Undo/Redo or reopen. Ordinary `asset_import` recognizes the same Wasm asset and
fingerprint.

The focused authored tests cover approval denial/acceptance, same-ID reopen and
service rebind/close, stale IDs/versions, candidate corruption, worker-versus-
ordinary-validator mismatch, source sibling edits before/after preparation,
cancellation, source-only output admission, exact retained node/membership
checks, bounds, aliases/links, ordinary reimport, repeated install preserving
asset identity, saved-project reopen and Undo/Redo after private candidate
removal. Mock compiler tests synthesize valid Wasm and exercise the actual
importer and journal; they do not prove compiler or sandbox execution.

Required later native selections include the source package tests, service
`install_tests`, agent `module_install` plus existing `module_registered` controls,
and strict all-target lint for affected packages. The separate real host route is:

```text
cargo test --offline --profile ci -p incant_agent --features host-qualification --test module_install actual::actual_local_compile_install_edit_rebuild_reinstall_and_candidate_free_reopen -- --exact --ignored --nocapture
```

That route performs two actual bounded Local builds, installation, and a source
edit changing the supported `Tick::log` effect from Info to Warn. The helper's
message is consumed by the callback. It requires distinct raw Wasm bytes and asset
fingerprints after registered recapture/rebuild with the installed sibling
present, while preserving the asset ID. After ordinary reimport and removal of
only candidates created by that test, reopen must retain the exact second asset
and bytes, Undo must restore the exact first asset and bytes, and Redo must restore
the exact second pair. The control does not execute the guest callback. It
requires trusted `INCANT_MODULE_BUILD_CONFIG` and an explicit CPU lease. Being
present, compiled or ignored is not a passing result.
