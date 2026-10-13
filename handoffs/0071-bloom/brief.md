# 0071 — Opt-in scene-linear bloom numerical and appearance review

Preparation only. No grant, GPU or target is assigned by this packet. Root alone
schedules execution after preceding leases release. Claude Opus 5.5 through ACP,
with **Max explicitly verified**, owns actual rendered review and styling. Astra
has completed CPU/source correctness, not appearance or efficacy acceptance.

## Source and ownership

Producer: `bee28593` on `codex/bloom`, based on main193
`195866988ee6286ecf301149ee3559feedddfb92`. Preserve the exact producer and inspect
`docs/spikes/scene-linear-bloom.md`, `evidence/bloom.json` and its source-review
receipt. Original output/tone, frame formats, camera/material/UI bytes are unchanged.
13 engine/test/manifest inputs are frozen, plus the reviewed technical document
retained before a documentation-only evidence/DRS qualification update.

Root may supply an explicitly reviewed integration commit. Record its exact
source diff and selected feature graph. Do not combine pending SSAO, DRS or GPU
timestamps by assumption. No default enablement, cooked guard removal, authoring
layout/art change or shader tuning without separately reviewed scope. Native
`cooked-output` refuses bloom; match default WGSL features on baseline/candidate.
Record unsupported compiled feature combinations as refusals, not successes.

No production per-frame pipeline compilation is allowed. Startup creates the
seven-pipeline catalog; each frame uses retained target handles and bounded
immutable parameters. Source and recorded binaries must remain paired throughout.

## Review contract

1. Run both new ignored device tests once through the guarded matched regression
   pass (no duplicate focused rerun). They cover nonzero isolated-emitter halo,
   zero-alpha residual RGB, resize/old-command lifetime, invalid configuration
   replacement, zero-gain exactness and pixel-camera bypass. They were **not run**
   by Astra. Any device validation/refusal is a real failure to preserve.
2. Inspect actual nonzero bloom using numerical HDR emitters with alpha 0, 1 and
   fractional coverage. A zero-alpha bright source must contribute zero, and
   fractional premultiplied extraction must scale with coverage. Include threshold
   below/at/above source luminance and a constant field at 1/4/6 levels. Do not
   infer efficacy from only zero-intensity invariance.
3. Compare complete fixed baseline/candidate frames for a compact real scene with
   opaque/BLEND emissive surfaces, ordinary lit material, sprites and world text.
   Screen-space UI and chrome/backdrop colors must retain the documented placement:
   UI is never extracted; world text is scene radiance. The coverage formula is
   deliberate, not a physical transmission/refraction model.
4. Cover perspective and orthographic, odd extents, offset/fractional rounded
   viewports, resize, and pixel-perfect bypass. Actual source extent controls
   filtering; no leakage from outside the scene rectangle into extraction. Check
   halo composition at transparent pixels and cropped chrome independently.
5. **DRS coupling is mandatory before claiming DRS compatibility.** This producer
   does not contain DRS. If root supplies that union, physical output position must
   first map into the internal scene coordinates before bloom lookup. Test coupled
   fixed 50/75/100 percent scales on odd dimensions/offset viewports with unchanged
   output/UI dimensions. Do not claim apparent-radius invariance: kernel footprint
   is intentionally internal scene texels. If no reviewed union is supplied,
   record DRS+bloom as pending and do not fabricate the result.
6. If root supplies timestamp integration, Display includes bloom before output
   without extra query slots. Record lifecycle/labels only; no timing gain claim.

The exact coverage/output formula and retained-memory exclusions are in the
technical document. Keep tone curve/chrome/sRGB bytes unchanged. Choose no new
look/style defaults in this correctness packet; any aesthetic proposal is a
separate Claude-owned result for root review.

## Budgets and safe runner

Before execution freeze a case list, per-case frame/submission counts, hashes,
binary names, exact arguments, cwd, feature sets and source commit. Use root's
explicit grant and target lease. Maximum two compile jobs. No native window,
audio, browser, desktop capture or accounts. Offscreen only.

- New fixture frames: at most **64 total**, including baseline/candidate controls,
  failed attempts, unretained frames and any allowed rerun. At most eight kept
  matched frame pairs (16 PNGs), maximum 960x540. The two new ignored device tests
  render ten frames total and are additionally charged to this new-frame budget
  even if invoked by the full suite, leaving at most 54 for other fixtures.
- Existing regression work: at most **2,048 rendered frames** total across matched
  variants; stop if preflight cannot bound a test within this ceiling. Thus total
  frame ceiling is **2,112**. Record actual test listings first: this producer has
  34 library + 50 model ignored tests; base has 32+50; GPU host adds 5+2 on each.
  Explain any integrated listing delta before execution. No blanket rerun.
- At most **16 serial GPU child launches**, including failures/reruns, each with
  a 300-second watchdog; new fixture children have a 120-second watchdog. Whole
  execution ceiling 45 minutes excluding compilation. Never loop on a failure.
- Reserve each child's full frame budget durably **before launch**; keep the
  reservation on failure/interruption. Finally kill/reap owned process groups,
  retain exit/error logs, and emit a release receipt only after actual PID/group
  absence checks. A failed test cannot disappear from the ledger.
- Use a constructed environment: system PATH plus explicitly pinned build tools,
  root-granted CARGO_TARGET_DIR, new packet TMPDIR, RUST_BACKTRACE and only the
  required packet output path. HOME/CARGO_HOME/RUSTUP_HOME may pass through
  unchanged only when necessary; never fabricate HOME or serialize secrets.
  Exclude ambient INCANT_*, WGPU_*, TRACY_*, CARGO_FEATURE_*, RUSTFLAGS, compiler
  wrappers and authored profile flags unless a specific recorded override is
  reviewed by root. No evidence variables for unrelated regression tests.

CPU preparation may list/validate helpers only when root authorizes target use;
this brief grants neither. Freeze source and actual executable hashes before
GPU launch. Use `tools/cargo --locked --profile ci -j2` and selected suites,
record compiler feature identities and test cwd. Do not edit another checkout.

## Return

Return `result.md`, frozen case/command/receipt ledger, actual adapter/OS and
features, raw numerical assertions and matched full-frame hashes, limited kept
pairs with your own appearance judgments, exact source/binary hashes, all
original failures and cleanup/release receipt. Include Clippy on any new helper.
Default output remains off until root accepts scoped evidence; no phone, overall
renderer, performance, MetalFX or temporal-upscaler gate is completed here.
