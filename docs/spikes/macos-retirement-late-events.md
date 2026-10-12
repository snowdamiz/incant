# Retirement probe late callbacks

Checkpoint13's general macOS job reached the actual invisible AppKit retirement probe, then panicked at `live hidden window`. The probe destroyed its window after cycle256 and requested event-loop exit, but a later queued callback still entered the cycle body. Production window handling is unchanged.

The probe now keeps completion and the first typed failure terminal. A helper contains the same native-cycle ordering and final active-owner destruction; late callbacks return exit before any owner access or native operation. Two deterministic regressions first reproduced the completion panic and error retry, then passed with the guard. They assert all256 cycles, one owner destruction and preservation of the first error.

The native-window feature's two tests, Clippy, actual example build, formatting, conventions and diff checks pass. The first missing error annotation and Clippy cleanup failures remain retained.

A copied immutable native-feature executable is ready, but the bounded idle gate found no20-second quiet interval and did not launch it. No screenshot, input injection or GPU action occurred. Actual AppKit execution is still pending locally and must pass in the comprehensive checkpoint before release. Checkpoint13 stays frozen with its failure; this repair does not relabel that run.

[Source, red/green logs, hosted failure and copied binary identity](evidence/macos-retirement-late-events.json).
