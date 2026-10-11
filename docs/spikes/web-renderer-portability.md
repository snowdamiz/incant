# WebGPU renderer ownership portability

The default `incant_render` library now compiles for
`wasm32-unknown-unknown`. The frozen main100 baseline produced 163 errors:
Bevy's `Resource` bounds rejected browser GPU handles, and the non-send source
inside `wgpu::Error` made every `ResourceError` fail the shared boxed
`Error + Send + Sync` boundary. The remaining field/method errors followed from
those invalid resource parameters.

Browser frames and command encoders now use Bevy 0.19.1 `NonSend`/`NonSendMut`
parameters and `World::insert_non_send`/`remove_non_send`. The private browser
graph also carries `PhantomData<Rc<()>>`: Bevy's `World` can move between threads,
so non-send storage alone does not express the graph's owner-thread boundary.
There is no unsafe Send/Sync implementation or fragile wgpu threading feature.
Desktop builds retain `Resource`, `Res`/`ResMut`, the graph mutex and their
existing Send/Sync contract.

The single renderer-initialization validation-error boundary detaches browser
diagnostics into an owned `PreparationError`. It preserves the out-of-memory,
validation or internal category, display message and copied source chain while
dropping the backend error owner. Native `PreparationError` is an alias for the
original `wgpu::Error`, including its typed source. The renderer and shared
`incant_render_data` result bounds remain unchanged.

The same six passes and dependency edges remain in force: scene initialization,
light assignment and shadows precede models; models precede sprites; sprites
precede display. Encoding stays serial. Frame and encoder handles leave the
scheduler before the completed command buffer returns.

## Local verification

Tests used the frozen source in this patch, Rust 1.99.0, wgpu 29.0.4 and Bevy
0.19.1, on an arm64 Mac running macOS 26.6 (25G72). Cargo used
`CARGO_TARGET_DIR=/tmp/incant-agent-native-target`, `--offline --locked`, the
`ci` profile and `-j2`.

- Actual default wasm32 library compilation passes. Its compile-time assertions
  require the browser graph to be neither Send nor Sync. Erasing only the Rc
  marker produced the expected two E0283 errors; restoring the exact source
  returned compilation to green.
- Native library tests pass: 36 passed, 18 GPU tests ignored. Four new CPU tests
  cover owner-thread pass ordering and repeated frame cleanup, retained native
  Send/Sync bounds, copied error category/message/source and backend-owner drop,
  and preservation of the native typed error source.
- Native all-target Clippy, formatting, actionlint 1.7.12 and 12 workflow/cache
  policy tests pass. The initial all-target run exposed standalone discovery of
  two module test files; placing them in module directories fixed that test
  packaging issue before the final checks.

The existing release-checkpoint web job now checks `incant_render --lib` beside
audio and runtime. No job, required context or full per-PR build was added.
[Exact commands, inputs and log hashes](evidence/web-renderer-portability.json).

These are CPU tests and compile evidence. No browser, GPU, window or rendering
acceptance was executed, and no shader, display transform or appearance changed.
Complete browser integration and device/performance gates remain open.

## Main102 integration

Reviewed source `593d0a0` merged actual main102 `8bd9a96` without conflicts as
`c8f6ce1`. All ten non-STATUS reviewed source/evidence hashes remain unchanged.
The integrated graph again passes actual default wasm32 library compilation,
36 native CPU tests (18 GPU cases remain ignored), all-target Clippy, 12 policy
tests, actionlint, formatting and conventions. No browser or GPU was launched.
The separate optional cooked-output implementation is not included here.
[Independent review](evidence/web-renderer-root-review.json) and
[integrated validation receipt](evidence/web-renderer-main102-integration.json).
