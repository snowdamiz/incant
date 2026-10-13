# Native-window CPU profiling on main204

The five independently reviewed production files from `94a97c6a9ad310bd3ab219016629abb0b5df47c0`
are byte-identical in this union. It retains actual main204 `5de3b848f6aebbbe1727db025639edb3aab701a6`,
including motion history and the reviewed compiler-ID repair. The only cherry-pick
conflict was STATUS prose. The producer's older lock lacks the already merged
renderer dev-only naga edge; the current lock is unchanged from main204.

Ten current commands exit zero: default player tests (20), native-window plus
Tracy library tests (69), actual offline profile CLI regression (one), summary
unit tests (four), all-target Clippy, wasm32 summary compilation, runtime dependency
guard, formatting, conventions and base diff. Counts overlap across feature builds.
All 2,889 frozen implementation/build files remain unchanged. The tested head
`69b77feb9e28df71a687dcfd0bad9bbcfe79e3b9` and ancestry head `3a7c4ccbc7ec1b6228d002dcc84f7430f59f8209` have identical trees.

The capture still starts only with explicit trusted host flags and matching
player features. Simulation attempts, presentation refreshes and restart work
have separate fixed CPU labels. Final report emission uses the existing callback
after successful native session/audio retirement. Errors preserve the prior
terminal handling and retire the capture without emitting a report.

The collector rechecks all 54 producer source rows, the 67-record/65-payload
producer archive, current logs and final independent producer review. Thirteen
archive rows are original external files and 54 are retained Git source copies.
No native window or Tracy client was launched by either this integration suite
or the producer suite. No GPU, appearance, shipping performance, phone or complete
profiler acceptance is claimed.

[Manifest](evidence/native-window-profile-main204.json) and
[read-only collector](evidence/native-window-profile-main204/collect.py) preserve
commands, source identities and lossless artifacts. Run the collector with
`--verify-only` to reassert its checks. Final integration review is pending.
