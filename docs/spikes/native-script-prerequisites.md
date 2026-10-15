# Native export compiler prerequisites

A real fresh cooked export reached the native build before discovering that its
engine checkout had no installed TypeScript dependencies. The exporter now checks
the selected scene before launching any build tool. If that scene attaches Script
components, its trusted compiler driver and the root-pinned TypeScript, SWC and
source-map packages must be installed at the expected versions. Missing or
mismatched packages report `npm ci` in the engine root as the repair.

Scenes without Script components, including Wasm scenes, do not need these Node
packages. Another scene's scripts do not impose that requirement on the selected
scene. The check neither installs dependencies nor imports project code; the
actual trusted cook still validates and uses the compiler implementations. This
is an early checkout diagnostic, not proof of package integrity or a replacement
for cook-time validation.

All 45 focused Python controls pass: prerequisite selection/refusal plus existing
Wasm routing, cooked-shader, profiling and package controls. The public-build
regression verifies failure before any child tool and without creating staging or
output files. [Command, source hashes and result](evidence/native-script-prerequisites/tests.json)
and [raw output](evidence/native-script-prerequisites/tests.log) retain the evidence.
No native build, driver, rendered result or performance gate is claimed by these
controls; the complete cooked-export qualification remains separate.
