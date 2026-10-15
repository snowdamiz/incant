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
pending publication; no local screenshot, browser, CUA, native window, actual
WebGL performance or appearance acceptance is asserted. The existing GLSL,
camera, authored fallback styling and all other site components remain unchanged.

Original CI log and exact local raw outputs/identities are retained under
`~/.codex/incant-integration-evidence/release-checkpoint-41/website-build-cancelled.log`
and `~/.codex/incant-evidence/website-render-admission-001/`. Bloom source
`2da0b57b` and its unexecuted CPU proposal remain on their preserved branch.
