# Fractional SSAO viewport diagnostic

Release checkpoint 26 failed the existing SSAO filter contract on Windows Dx12,
using Microsoft Basic Render Driver 10.0.26100.33438. The horizontal result at
pixel `(17, 1)` was 255 rather than the unchanged scalar interval 98–99. All
rendered depth bits had already matched the fixture before that assertion.
The failure is retained; no release promotion or production fix is claimed.

The same fractional viewport is used by production SSAO postprocessing. The
new ignored diagnostic keeps the original shader, input textures, uniform
rectangle, depth checks and interval oracle, and observes three raster modes:
the original white clear, a fractional viewport with a 17/255 clear, and a
full-target viewport with that same sentinel clear. The shader still evaluates
the original rectangle. This separates a potentially unwritten pixel from a
white rejection result and tests whether coverage changes the discrepancy.

Perspective and orthographic cases use six 19×15 submissions, six maps and
eighteen render passes in total, without images. Logs include both stage hashes,
inside/outside mismatch counts and the failing pixel's input/output values.
Diagnostic completion is explicitly observation-only. The existing strict test
still runs its original two cases and retains every expected interval.

The manually dispatched `Renderer numerical diagnostic` workflow runs only
this exact test on Windows, requires its actual Dx12 adapter and all observation
markers, and saves the exact commit, toolchain, output and exit status. It uses
a read-only dependency cache and does not replace or waive any release check.

Local CPU compilation, the fixture's discriminating reference test, scoped
strict Clippy, formatting, convention and workflow checks are recorded in the
[local evidence](evidence/ssao-viewport-diagnostic.json). Actual diagnostic GPU
observations remain pending. The original Windows failure alone does not yet
identify whether raster coverage or shader arithmetic caused the white pixel.

The diagnostic is composed on main238 with both reviewed source files byte-exact.
A fresh scoped current-main CPU run passes its one discriminating fixture case;
both GPU tests remain ignored locally. Current library and test dependency graphs,
formatting, conventions and full-base whitespace checks pass. The test graph
contains ordinary deterministic Rapier without checked-restore or the inert
snapshot crate. The original strict Clippy and actionlint evidence remains scoped
to the producer. The [composition record](evidence/ssao-diagnostic-main238.json)
binds all 3,280 build inputs and six current checks. Initial runner PATH and
collector graph-classification errors are retained; neither changed source or
repeated a passing CPU test.

The workflow is manual-only. Later broad ignored-test release invocations also
discover these six observation submissions alongside the unchanged strict test;
observation success does not establish oracle success or repair checkpoint26.
