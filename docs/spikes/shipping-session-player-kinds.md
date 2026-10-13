# Distinguish player library and executable in fresh shipping evidence

Checkpoint16's general macOS job failed after the successful ordinary player
build. Its fresh-session ledger selected rustc commands by crate name/features,
then exempted executables from the library suffix filter. Since the player now
has both library and executable targets, two commands matched the executable.
The earlier standalone exporter fix did not cover this separate ledger.

The ledger now parses explicit rustc crate types and compares them with Cargo's
artifact target crate types before the existing feature, suffix and unique-command
checks. Missing or duplicate compiler crate types refuse. Exact artifact identity,
fresh executable compilation, prior-session proof, hash verification and deletion
barriers remain enforced. No runtime, shader or workflow source changes.

The original hosted failure and all six raw Cargo streams are preserved in the
[manifest](evidence/shipping-session-player-kinds.json). Three regression controls
cover distinct library/executable commands, rejection of library-only executable
proof and duplicate executable commands. Unchanged source reproduces the first
two failures; the correction passes all24 ledger tests. Full tools discovery
passes330 cases with four existing Windows-only skips. Conventions and diff checks
pass. The independent source review reruns the24 focused tests successfully.

The exact hosted command replay shows library1→1 and executable2→1 candidates.
It proves the command-selection correction against the actual failing input,
not a rebuilt shipping package. A future frozen comprehensive checkpoint must
still complete the actual fresh-session builds and source-deleted players.
Checkpoint16 remains frozen and unpromoted; all23 required contexts stay required.

[Independent review](evidence/shipping-session-player-kinds/independent-review.json)
SHA-256 `66fc6caba81bb4c8d71beac2b99c4587a098f25b48a485e5b2bf3be2f237ad53`.
