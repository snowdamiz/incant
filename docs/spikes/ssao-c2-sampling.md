# SSAO sampling distribution candidate

This implements Claude0070's C2 prescription from `766dc101`, section 4 of
`next-repair-preparation.md`. Rotation uses the existing Bayer stratum. A
bit-reversed stratum and a per-ray digital Sobol shift distribute the maximum
64 probes across 64 radial strata instead of revisiting four shared distances.
The independent CPU model generates the digital coordinate from direction
numbers, rather than using the shader's lookup table.

The 16 directions, four steps, shading-normal sign, visibility predicate and
plane-aware five-tap filter are unchanged. The uniform remains 736 bytes and
logical targets remain 14 bytes per pixel. There are no new passes, allocations,
bindings, pipeline variants or default enablement. Extra integer/phase arithmetic
has not been timed; equal probe counts do not establish equal execution cost.

The CPU model passes the unchanged 0.25 stripe-variation bound at the previously
selected perspective/orthographic rows and their six adjacent rows. These are
14 model measurements, not device results. Its predicted margin at the original
perspective row is less than one output code, so actual GPU confirmation matters.

The original adjacent-gradient metric N6 remains failed. A separately reviewed
experimental measure N6T compares the local gradient with an independently
converged smooth reference, on the original 70 perspective pairs and with the
same 1/64 bound. An independent finer smooth reference fails raw N6 after R8
storage but passes N6T; measured old banding, synthetic bands/hatch and a four-code
extra step fail N6T. This explicitly revises the metric rather than retroactively
passing N6. Orthographic N6T remains a failing diagnostic. Both values and the
original failure history must accompany the next visual result.

Scoped qualification passes 105 default library CPU tests and 119 observer/cooked
CPU tests across all targets (116 library plus three integration cases), strict
all-target Clippy, wasm32 check, formatting, conventions and diff checks. GPU
tests remain ignored. The first run retained an old R1/R3 test's literal expected
failure; it stopped with 104 passing and one failing test. The corrected test
keeps the original geometry and bound and checks all 14 C2 model rows, without
changing production source. Both attempts are retained.

Independent source review accepted the original three-file delta and the final
test-only followup. The [manifest](evidence/ssao-c2-sampling.json) retains exact
source copies, raw command logs, the metric review and copied CPU test binaries.
Actual shader execution, stage-oracle agreement and Claude's rendered appearance
review are still required. No shader-quality, physical-device, performance or
phase gate is approved by this candidate.
