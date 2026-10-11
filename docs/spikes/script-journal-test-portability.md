# Script journal test portability

The initial release checkpoint PR85 at frozen main84 `b84a9bf` failed its
Windows desktop workspace test job114516221927 (run38155060407) after11m04s.
`journal_replays_registration_replacement_removal_and_original_actors` tried to
read the journal through a separate filesystem handle while its persistent
CommandBus still held the exclusive lock. Windows returned OS33. The failure
and raw log identity are retained; this was not a passed checkpoint.

The test now drops the owning bus before each exact-byte read and reopens the
same journal with the original project afterward. It explicitly checks the
reconstructed project, history, revision and empty redo state before continuing.
The refused removal still must preserve the exact journal bytes; registration,
replacement, removal, actor provenance and all later Undo/Redo/replay assertions
remain. No production lock, journal or command behavior changed, and no retry was
introduced.

Independent root review accepted the narrow fixture correction. All six script
command tests and their test-target Clippy pass locally on macOS ARM64. Actual
Windows confirmation remains required in the next complete frozen candidate.
The repair follows the new policy through reviewed local verification and the
fast main gate; PR85 is not promoted or silently rerun. [Exact evidence](evidence/script-journal-test-portability.json).
