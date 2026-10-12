# Player library and executable shipping audit

The reusable player library and thin executable share `incant_game_player` as
Rust's crate name. The ordinary exporter previously required one target compiler
invocation for that name, so the first fresh main169 Planar export failed after
compilation and before package publication.

The audit now selects each required crate name and crate kind. It checks the
player library and executable independently for shipping optimization, codegen
units, aborting panics, debug symbols, target portability and the absence of the
source compiler/test harness. Fat LTO is required on the final executable. The
existing executable receipt key is preserved; `incant_game_player_library` records
the additional audit. Runtime/script libraries and authoring dependency checks
remain mandatory. This does not alter compiler flags or relax the export gate.

The old source rejects a recorded real-shaped lib+bin command sequence. Six new
parser regressions cover both units, missing/duplicate commands, host-vs-target
selection, compiler/test-harness leakage, authoring target compilation and bad
shipping flags. The scoped native tooling suite passes 63 tests; four Windows
subprocess tests are correctly skipped on macOS. Synthetic log controls establish
the parser behavior, not shipping or cross-platform runtime success. The actual
failed export receipt is retained. Corrected fresh public export execution is
still pending and will be recorded by the consuming Planar proof.

[Commands, hashes and red/green receipts](evidence/native-export-player-units.json).
