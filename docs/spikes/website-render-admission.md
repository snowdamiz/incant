# Website raymarch admission and fallback

Checkpoint 41 website job `116229023408` in run `38675810487` reached its
ten-minute job timeout. Its retained log shows the desktop/tablet composition
cases timing out at about 96 seconds, narrow/mobile cases taking over 80 seconds,
and ordinary navigation/accessibility cases timing out. It was cancelled before
the final detailed test report. The log does not identify the WebGL backend, GPU
timings or a particular failed assertion, so software rasterization remains a
source-supported hypothesis rather than a reproduced cause.

The renderer accepted the default `failIfMajorPerformanceCaveat: false`, including
potentially very slow contexts, before compiling full-screen raymarch programs
with up to 140 traversal iterations. Resolution adaptation waits for eight slow
frames and cannot prevent expensive initial context/work admission. The
[WebGL specification](https://registry.khronos.org/webgl/specs/latest/1.0/#WEBGLCONTEXTATTRIBUTES)
defines `failIfMajorPerformanceCaveat` specifically for implementations whose
performance is dramatically below native rendering, including software paths.

The runtime now requests that admission policy once and uses the existing
authored fallback if the browser refuses or throws. It does not retry with weaker
attributes, detect CI/user agents, change shader arithmetic, lower design quality
or skip site tests. Failed shader/link/buffer preparation releases partial
resources and the optional context before fallback. Successful disposal is
idempotent and cannot be revived by an already-queued observer callback.

The existing browser test helper also had a concrete fallback defect: it tried
to scroll a hidden canvas into view before checking whether fallback had hidden
it. It now scrolls the visible scene container and requires a visible fallback
when the canvas is hidden. All existing composition, accessibility, interaction,
asset-path and screenshot assertions and their time limits remain. One new
hosted browser case deliberately refuses WebGL and verifies that every scene
settles through the fallback and ordinary content remains available.

Local source-only qualification used Node 22.23.3 / npm 10.9.9:

```sh
cd website
npm ci --offline --ignore-scripts
npm run test:unit
npm run typecheck
PAGES_BASE_PATH=/incant/ npm run build
```

Seven Node controls passed against the actual renderer function with plain
mocked canvas/GL/observer objects: refused/throwing context admission, partial
fragment/link/buffer failure cleanup, successful animation/disposal, and
reduced-motion still/invalidation. They do not create a browser or graphics
device. The exact unit-file command fails if its input file is missing. Strict
typecheck, production build, conventions and diff checks also passed. A preliminary
wrong test-file directory produced zero tests and a separate browser-test `this`
type error; neither was counted as a passing qualification. Both were corrected
before the recorded seven-test pass.

The website workflow runs these CPU controls before its full existing Playwright
suite. The local Playwright suite was intentionally **not run** because it takes
screenshots and local capture remains prohibited. Hosted browser confirmation is
described below; no local screenshot, browser, CUA, native window, actual
WebGL performance or appearance acceptance is asserted. The existing GLSL,
camera, authored fallback styling and all other site components remain unchanged.

Original CI log and exact local raw outputs/identities are retained under
`~/.codex/incant-integration-evidence/release-checkpoint-41/website-build-cancelled.log`
and `~/.codex/incant-evidence/website-render-admission-001/`. Bloom source
`2da0b57b` and its unexecuted CPU proposal remain on their preserved branch.

## Hosted attempt and bounded fallback successor

The first published repair at `049e10ab` also reached the ten-minute job limit
in run `38678645222`. Three smaller composition cases completed, while desktop
and ordinary interaction cases still timed out. The new forced-refusal case
also failed; source review found its GitHub locator matched both hero and footer
links. It now checks the intended hero link. The original cancelled run is
retained and is not counted as a successful website check.

The successor refuses known software renderer strings (SwiftShader, llvmpipe,
softpipe and Microsoft Basic Render Driver) before compiling any shader. That
identity is used locally only, never stored or sent. If private renderer identity
is unavailable, normal admission remains possible. A visible animation frame
interval over 250 ms retires the optional context and reveals the existing
fallback instead of queuing more work. Offscreen/hidden intervals reset the
watchdog. This is a conservative presentation policy, not a measurement of GPU
execution time or proof that software rendering caused the original timeout.

Thirteen Node controls, strict TypeScript and the `/incant/` production build
pass. A nullable closure type error during the change was corrected before this
qualification. The additional controls cover software refusal before compilation,
single fallback/disposal after a stall, and visibility-resume behavior. Browser
failures now print immediately so a later job timeout cannot hide their messages.
All existing browser tests, screenshots and deadlines remain. The new full
hosted run is still required before merge; this Mac launched no browser or capture.
[Exact source and retained logs](evidence/website-render-admission/results.json).
