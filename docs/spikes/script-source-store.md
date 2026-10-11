# Script source store prerequisite

The authoring-only `incant_source` crate supplies confined filesystem operations
for the shared command bus. It is not an engine-agent tool or a source editor.
`check_recovery(root)` reads without creating files, refuses a pending operation,
and returns an opaque `SourceEpoch`. Standalone compilation compares epochs before
and after graph acquisition to detect engine transactions crossing that capture.
External editors do not participate in the store lock or epoch protocol.

The store stages one UTF-8 `.ts` file of at most 256 KiB. Source paths are relative,
portable and resolve through existing no-follow directory capabilities. Hardlinked
files, symlinks/reparse points, reserved paths and missing parents are refused.
No-clobber moves retain the actual displaced node before checking its contents;
foreign data is preserved and recovery remains blocked. These are conflict checks,
not a filesystem compare-and-swap promise. The first slice does not garbage-collect
retained files. Its state directory is engine-owned transaction state.

On Linux/macOS, operations use directory handles and `renameat` NOREPLACE plus
file/directory sync. Windows pins each directory with a no-reparse handle denying
DELETE sharing, and uses no-clobber, same-volume `MoveFileExW` WRITE_THROUGH plus
write-through synced files. Windows has no portable directory `FlushFileBuffers`
equivalent; these tests establish process-crash behavior, not a physical power-loss
certification across storage devices. The live pathname may briefly be absent
between displacement and installation; engine consumers must use readiness checks.

Local macOS validation: nine actual filesystem tests passed, including child
process exit without Rust destructors after marker preparation and installation,
retained external edits, create races, path/link refusal, exclusive root locking,
and read-only epoch checks. `cargo clippy -p incant_source --all-targets --locked
--profile ci -j2 -- -D warnings` passed. A read-only independent protocol review
found no ordering blocker. The `Script source filesystem transactions` workflow
runs these tests and Clippy on macOS, Linux and Windows, including a real Windows
junction test. Linux/Windows execution remains CI evidence to collect, not a
claim from local compilation or this source review.

The bus/journal/tool integration and fuller crash-boundary tests are a following
scoped change. This prerequisite alone does not expose script content authoring.
