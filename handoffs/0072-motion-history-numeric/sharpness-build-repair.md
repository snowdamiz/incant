# Sharpness build evidence correction

The first supervised build in `0072-fixed-sharpness-001` compiled both the
library tests and visual fixture successfully. Its helper then refused because
`cargo test --lib --no-run` emitted no ordinary (non-test) renderer library.
The empty set was incorrectly compared with the integration fixture's rlib.
The original stop, outputs, binaries and released target lease are retained;
no GPU ran.

The helper now checks exact source and the actual unit-test executable's package
and feature identity against the fixture's linked library. If the unit-test
build emits an ordinary library, its bytes must still match exactly. An absent
ordinary library is explicitly `null`, never claimed byte-equivalent. Eight
CPU controls reject missing/duplicate artifacts, wrong source/features and
different emitted bytes. Preparation self-check passes.

A separate `0072-fixed-sharpness-002` attempt uses unchanged engine a143b5c0,
fixture, count limits, quality thresholds and baseline. Root supervises its
build, strict Clippy and later GPU readiness. The baseline's known halo and
same-phase threshold failures stay recorded; no threshold has been relaxed.

Built-by: astra
