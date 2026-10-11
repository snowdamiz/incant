# Native assets and renderer integration

The combined tested source is `ef2897af60d74efdd193288108206d4858ef75ae`.
It preserves corrected FBX head `8cbda8350b76ec213902d564b5bb1e8af87afecd`
and renderer/flipbook head `1f56e92dc0efdfbb823189b38004d224859adb34` on
verified main80 `611d867bee1ff3c8f33da8c221c82867408e96c6`.
Only STATUS conflicted; no implementation conflict resolution was needed. The
GPU test module includes both pipeline prewarming and generated-tangent suites.
Asset and runtime-asset sources remain identical to accepted FBX source
`f2963ba9c1d8a98a9957074910296099b2e0c477`. Original author trailers are preserved.

One combined publication replaces separate FBX and renderer follow-ups, avoiding
an otherwise required later main refresh and duplicate hosted matrix. Original
PR77 stays held until exact corrected ancestry is verified in main. The
[foundation checkpoint](foundation-integration-queue.md) records the prior merges.

## Local verification

All checks below passed on the frozen combined source:

- 685 ordinary workspace tests/doctests and all 73 serialized ignored GPU tests
  in the adopted `ci` profile, with the scene-audio feature.
- Both workspace all-target Clippy profiles (`ci` and development), plus optional
  native-window all-target Clippy and 12 actual policy/routing tests.
- 536 UI tests in 30 files, strict TypeScript/production build, 91 tooling tests,
  SDK typechecking, formatting, conventions, bridge parity and actionlint 1.7.12.
- Exact regeneration parity for all 58 schema files, the full normal-dependency
  guard including optional native-window and planar physics, and no ufbx in the
  browser normal graph.
- wasm release build/Clippy and an actual Chromium 156.0.8078.4 smoke through
  wasm-bindgen 0.2.126: all seven source-model refusal routes and the existing
  120-tick physics/input/navigation checks pass. This CPU browser run disables
  GPU and is not appearance/device evidence.
- Eleven public CLI FBX processes verify imports, unchanged cache hits, stable
  reimport identity, Undo/Redo, atomic invalid/unsupported refusal and validation
  after deleting source files. The existing helper was copied to `/tmp` with only
  its binary path changed from `target/release` to the integrated `target/ci`.

The fresh `native-game.py --gpu --flipbook` shipping proof uses an isolated target
and the declared shipping codegen, then removes source/authoring/build inputs.
The resulting aarch64 macOS player is **8,960,176 bytes**, SHA-256
`d02e82a0c7d5785458422c3f33a8474cd30a6e329ca601f173aa982d9bb18a3a`.
It passes 32 repeated replay/GPU/PCM processes, 35 refusal cases, four actual
animated-versus-static frame comparisons, and the existing plain-v1, CPU-mode,
observation and log controls. These are categorized controls, not a claim that
32+35 is the total process count. Numerical rendering used Apple M5 Pro/Metal;
there is no on-screen presentation, live device audio or phone performance claim.
The proof process exited successfully and no child player/build process remained
when the GPU slot was released at 06:30:12Z.

The [machine-readable report](evidence/native-assets-render-union-local.json)
retains exact results, binary/source identities and log hashes. Primary logs are
`/tmp/incant-assets-render-union-*`; the fresh shipping record is
`/tmp/incant-assets-render-union-shipping/result.json`.

## Bounded tooling follow-up

After all runtime checks were terminal, the union preserved reviewed commit
`e20bbbee889c307bc5e0a6498ba4df47bf8865bc`. Main80's Linux composed-Wasm
producer had failed before compilation on a WABT TLS connection reset; preceding
static conformance and standalone shipping checks had passed. The provisioning
helper now retries only bounded transient transport failures, at most three
attempts with 0.5/1-second backoffs. The original 30-second per-call timeout is
not a total deadline. Pins, URL, transfer/member limits and certificate/integrity
refusals are unchanged. [Full scope and fault evidence](wabt-download-retry.md).

The final 96 tooling tests pass, including actual localhost fault controls.
A complete changed-path audit confirms that engine/compiler/asset/render source,
build inputs, manifests, toolchains and workflows are byte-identical to the
fully tested `ef2897a` source. Only this provisioning helper/test and evidence
changed. No engine rebuild was needed for that reviewed tooling-only delta;
hosted execution remains a separate requirement.

## Appearance and remaining limits

Claude Opus 5.5 with explicitly verified Max accepted the respective frozen scopes:
[0040 static FBX/tangent delta](../../handoffs/0040-static-fbx-import/tangent-delta-result.md),
[0041 prewarming](../../handoffs/0041-prewarmed-pipelines/result.md), and
[0042 flipbooks](../../handoffs/0042-native-flipbook/result.md).
Root reviewed this unchanged source composition and required no new appearance
route. Original capture identities and the prior 0040 late-overlap disclosure are
preserved; no timing claim uses those captures.

The full hosted classifier and all required cross-platform/shipping checks remain
enabled and must pass for this final head before merge. Static FBX does not claim
animation, skinning or full FBX compatibility; browser source-model cooking is
explicitly unavailable. New recipe identities require reimport/recook to correct
old generated tangents, while old immutable cache versions remain readable and
retain their old appearance. The separately identified OPAQUE/emissive/environment
alpha-weighted mip defect is not fixed by this union. Complete flipbook SDK/save,
reference-phone, performance, export, full engine/game and phase gates remain open.
