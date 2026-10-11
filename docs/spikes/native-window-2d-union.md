# Native window and 2D integration

This union preserves the reviewed ancestry of native window PR63, tilemap data PR68,
tilemap rendering PR73 and native planar physics PR78 on the native project-play
PR79/PR76 foundation. It replaces four divergent integration heads with one source
and validation boundary. Original author trailers, scoped limitations and Claude
acceptance evidence remain intact. FBX, pipeline prewarming and flipbooks are not
part of this union.

## Resolutions

CI retains the adopted `ci` profile, fail-open full-check classification, normal
runtime dependency guards and all source-free shipping/probe commands. The optional
`native-window` feature includes `render-gpu`; its all-target test command now also
executes the real CLI routing regression. No coverage is removed to make the union
pass.

The player routes `--window` before headless observation parsing. Unsupported
`--observe` in window mode therefore fails at argument parsing, before absent scene
files or device setup; headless mode still reaches its typed observation parser.
The two-process regression proves both routes. Game lifecycle methods retain the
reviewed take-before-sync terminal behavior, while constructor signatures continue
to accept either supported `SessionProgram`. Runtime exports include both native
planar physics and the revocable 3D query API. PR79 merged without conflicts and
keeps its shared typed observation crate and source-free native-project route.

Only STATUS/Cargo/workflow/entry-point/export integration conflicts needed
resolution. No new renderer math, shader, simulation policy or appearance design
was introduced. The initial public tool build in the fresh worktree reported a
missing UI dist directory; building the unchanged UI and rerunning that command
succeeded.

## Verification

The frozen implementation is `07c0d43316f7c374fb6989f313999fde12b74176`.
Detailed command logs, source-tree IDs, executable identities and process reports
are in [the local evidence ledger](evidence/native-window-2d-union-local.json).
The final evidence commit changes only this ledger and the current status report.

- 650 ordinary workspace tests/doctests and all 69 ignored GPU tests pass. GPU
  execution used Apple M5 Pro/Metal with serialized device lifetimes.
- Workspace Clippy passes in both `dev` and `ci` profiles. The optional native
  window passes 18 policy/routing tests and all-target Clippy.
- All 534 UI tests, strict TypeScript, production UI build, 90 tooling tests,
  unchanged generated schemas, formatting, conventions, workflow lint and normal
  runtime dependency guards pass.
- Static module composition passes 101 tests plus Clippy, followed by a fresh
  shipping executable with eight successful and 26 refusal processes.
- Eighteen native one/four-worker and actual browser serial streams match for
  both 3D and planar physics. A separate fresh planar shipping executable passes
  three successful and four refusal processes after temporary build inputs are
  deleted.
- Native save restart controls and six custom native-project playback processes
  pass, including false observations, terminal failure and source-digest refusal.
- The fresh 8,893,968-byte GPU player passes repeated actual frame/PCM controls
  for model and sprite scenes, 33 refusal cases, CPU-mode controls, bounded logs
  and typed observations after authoring/build inputs are deleted. The report
  retains its exact symbol/codegen and executable identity; size is not a phone
  or isolated-feature performance measurement.

## Remaining boundaries

Claude0037 owns the native-window acceptance recorded in
[native-window-player.md](native-window-player.md), and Claude0039 owns the tile
appearance acceptance in [native-tilemap-rendering.md](native-tilemap-rendering.md).
Their evidence is pinned to its original source/executables; this union does not
substitute an Astra appearance judgment. Hosted exact-head results remain required
before merge.

Legacy Play continues to refuse tilemaps. Native planar physics retains explicit
root/unit-scale/planar constraints and does not add 2D characters, joints, complete
script SDK coverage or physics/composite saves. The native window retains its
platform and recoverable-surface limitations. Full editor Play migration, flipbooks,
reference-phone budgets, complete export and phase gates remain open. Allocation
measurements describe bounded CPU staging only, not the whole draw/driver pipeline.
