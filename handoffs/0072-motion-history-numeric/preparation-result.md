# 0072 stage B result — temporal visual review preparation

**Status: PREPARED.** The visual review is ready for a future root grant. Nothing
in this stage was built, captured, rendered or run against a Cargo target, and no
GPU process was launched. The future evidence directory
`~/.codex/incant-evidence/0072-temporal-visual` does not exist, and there is no
grant.

**Model and effort:**
- Model: Claude Opus 5.5 (`claude-opus-5-5`).
- Thinking effort: max.
- Both values are as this session reports them.

**Director feedback (revision 5).** I applied it as follows:
- I moved from targeted source reads straight to a small, complete fixture, with
  no new repository survey.
- I reused 0068's reviewed helpers, plus small adaptations the brief requires.
- I made no production source or shader change.
- I made no memory writes.

Stage A's GPU release preceded this work; see `temporal-result.md`.

## Frozen identity for root's grant

| Item | Value |
|---|---|
| Visual tree (`git rev-parse <commit>:handoffs/0072-motion-history-numeric/visual`) | `ac8a48de3a4df9314bee210c6e3c050172ec4862` |
| Fixture `visual/review/temporal_visual_review.rs` | 2,055 lines, `ec804d60…4fb2` (rustfmt 1.10.0 clean) |
| Plan `visual/plan.json` | `6af62fba…aa2f` |
| Tools | `common.py` `1d2106f1…`, `lease.py` `9d64f829…`, `build.py` `18ec6b00…`, `gpu.py` `f49faf60…`, `analyze.py` `027e028b…`, `selfcheck.py` `9d0003c2…`, `test_lifecycle.py` `98c2199c…` |
| Reviewed source | `e73afe43ffaf78e73b725876411128d092791f5a`, freeze `06a242fd…` (57 files) |

### What `review-ready.json` must contain

The grant goes in `~/.codex/incant-evidence/0072-temporal-visual/review-ready.json`
and needs these fields:

- **`"packet"`:** `"0072-motion-history-numeric"`.
- **`"stage"`:** `"temporal-visual"`.
- **`"source_commit"`:** any commit whose build inputs are byte-identical to
  `e73afe43`. That is the case for HEAD and for this commit. `build.py checkout`
  refuses otherwise.
- **`"visual_tree"`:** the tree hash above.
- **`"target"`:** a dedicated `~/.codex/<name>-target`.
  - Root's 0068 target and the shared `/tmp` CPU target are refused.
- **At least one `prior_<name>_release_path` / `_sha256` pin.** For example, the
  r3 release `0072-temporal-r3/gpu-released.json`, `0c63e503…`.

## Fixture composition (my quality ownership)

Each case is an ignored test in one binary, copied into the isolated `git
archive` checkout only. Every case renders at 960×540 through
`RetainedScene::submit`.

The explicit, validated config is `REVIEW`:

| Field | Value |
|---|---|
| `history_weight` | 0.9 |
| `relative_depth` | 0.05 |
| `absolute_depth` | 0.02 |
| `reactive_threshold` | 4.0 |
| `reactive_strength` | 0.5 |

Why these values:
- **History weight 0.9** converges within the 8-phase cycle and bounds lag.
- **5% relative depth** admits the per-pixel depth gradient of grazing floors at
  540 lines.
- **Reactivity** engages only for large scene-linear changes, so it does not
  undo texture-edge anti-aliasing.

`probe` checks that the config validates and that five invalid ones are rejected.
There is no preset.

| Case | World frames + UI | PNGs | Coverage |
|---|---|---|---|
| `probe` | 0 | — | Adapter/backend, temporal enable and idempotence, config validation, timing constructor |
| `stationary` | 17 | off, on-f16, activity map | Two full jitter cycles on a look-dev stage: grazing ground, silhouettes, a thin post and wall, 19 receding fence posts, and a one-texel **MASK** grille card |
| `motion` | 16 | 4 | **Object motion and disocclusion:** a cube at about 21 px/frame across the thin wall towards the sphere. **Camera pan:** a fresh owner and parallax. Each has an off reference |
| `excluded` | 9 + 9 UI | 3 | Accumulating opaque checker, with current-only **BLEND** card, **sprites**, **world text** and a screen-space **UI** button over uniform sky after every frame |
| `transitions` | 13 | 2 | Explicit **cut**, fixed half-scale **DRS** (internal 480×270), return to output scale, one owner throughout |
| `skinned` | 7 | 2 | The skeletal fixture from the passing motion test; clip 1 rotates the second joint so two vertices deform, against a checker |
| `timing` | 8 | — | Timing device. Off/on owners interleaved four times; every frame must yield a well-formed sample. **Median Models span must grow** when motion, resolve and copy are encoded inside it |

**Totals:**
- **Submissions:** 70 world frames + 9 UI = **79 of ≤96**, reserve 17.
- **PNGs:** **14 of ≤16**, reserve 2.
- **Counting rule:** every world-frame and UI submission counts, including
  references and unkept frames. Readback is part of its frame.
- **Regression:** the existing 96 ignored cases (38 lib, 50 `model_gpu`, 1
  timestamp probe, 5 game, 2 ui) run once, are never retried, and are counted
  separately.

**How the fixture decides pass or fail:**
- **Contract checks** decide each case's outcome. They cover only behaviour the
  passing numeric tests already established:
  - phase sequence, history presence, epoch, extent and owner;
  - UI pixels within 2 sRGB steps across all jitter phases and the off frame;
  - scene content present;
  - timing sample shape;
  - Models-span growth.
- **Image metrics are observations only.** These are edge/interior split diffs
  against the off reference, same-phase residuals and activity maps. My personal
  inspection of every retained PNG makes the appearance judgment.

## Concrete pre-execution finding (source, not yet observed)

**Expected weakness at depth edges.** `resolve.wgsl:43-47` rejects history for a
pixel if **any** of its four bilinear taps fails the depth test, including taps
with zero weight. For static content the motion is zero, so one current-only
band forms beside every depth edge: silhouettes, 1-px fence posts and MASK
cut-outs.

**What that would look like.** Under jitter those pixels should shimmer
frame-to-frame, while the unjittered temporal-off image is stable. Accumulation
would then add edge shimmer while smoothing interiors.

**How the review tests it.** The `stationary` activity map and the edge/interior
splits measure exactly this.

**Status.** This is a hypothesis to verify, not a measured result. I propose no
shader change. Any change would follow an observed result and Astra's
implementation.

The final assessment must also cover:
- **Current-only jitter:** BLEND, sprites and world text by design.
- **DRS upscaling:** nearest-neighbour.
- **Scope:** this is fixed internal-resolution accumulation only, not temporal
  upscaling or MetalFX.

## Helpers and the brief's essentials

The helpers are adapted from 0068's reviewed `common`/`lease`/`build`/`gpu`
helpers. This is not a new framework. Each essential maps to an implementation:

- **Immutable linkage.**
  - The grant pins the source commit and visual tree, and `tools_identity`
    refuses dirty or different trees.
  - The checkout is a `git archive` export, with every blob and mode verified.
  - Build inputs must equal `e73afe43`, and the 57-file freeze is re-verified.
  - Binaries are picked by Cargo's JSON messages, never by file name. They are
    frozen at `0555`, and their hash and execute bit are re-checked before each
    child.
  - The fixture copy must equal the committed fixture byte for byte.
- **Minimal environment.**
  - `HOME` is passed through without being recorded.
  - Everything else is a recorded allowlist: `PATH`, `TMPDIR`, `RUST_BACKTRACE`,
    target and Cargo flags, `INCANT_EXPECT_GPU_BACKEND=Metal` and the review
    output directory.
- **Global `native-gpu.lock`.**
  - It is held non-blocking around every GPU child.
  - The review debit (frames, UI submissions, PNGs) is fsynced under the same
    lock before launch, is monotonic and is never refunded.
  - Debits go to their own `budget.jsonl`. Ledger rows always carry `pid`, which
    avoids the r2 `KeyError`.
- **Kill and wait.**
  - INT, TERM and HUP are blocked except during the wait, and the child starts
    with the default mask.
  - Each child gets its own process group, and that group is always
    `SIGKILL`ed and reaped.
- **Stop on failure.**
  - Any failed build or child latches `stop.json`, and every later step refuses.
  - Review cases run once each, in plan order.
  - `gpu.py release` writes the release sentinel, and `lease.py release`
    follows.
- **Evidence preservation.**
  - Outputs, captures and copies are opened exclusive-create, so nothing is
    overwritten or reused by name.
  - `analyze.py check` re-hashes everything.
  - `publish` copies at most 16 PNGs, and only after both releases.

## Commands run and results (CPU only)

| Command | Result |
|---|---|
| `python3 -I -B visual/tools/selfcheck.py --write visual/preparation/selfcheck.json` | `all_pass: true`. Plan totals and budgets match; the fixture matches the plan (test order, `Report::open` limits, 14 PNG names, 960×540, output variable, `REVIEW` fields). All 15 guarded commands refused with the grant absent. No evidence directory was created, `git status` was unchanged and the global GPU lock was untouched. |
| `python3 -I -B visual/tools/test_lifecycle.py --receipt visual/preparation/lifecycle.json` | **14/14 passed** in 2.65 s, using real `/usr/bin/true`, `/usr/bin/false`, `/bin/sleep` and `/bin/sh` children with temp paths and a temp lock. Results below. |
| `rustfmt --edition 2024 < fixture` (stdin; no build, no target) | Parses cleanly. I applied its formatting, and the file is now clean. |

The lifecycle tests verified:
- the debit is written before launch;
- timeouts and failed exits are killed and reaped, and latch stop;
- SIGINT, SIGTERM and SIGHUP each produce kill and reap, a preserved exception
  and a still-charged attempt;
- the child starts with the default signal mask;
- non-executable binaries, budget exhaustion, a busy GPU lock and a latched stop
  are each refused before anything is charged;
- the same holds for builds: success, timeout and SIGTERM.

**Type review.** Building was not permitted, so I reviewed every call, field and
import against the source.
- I found and fixed one certain compile error: `rect_delta`'s untyped
  `let mut worst = 0` followed by `.max()`, which is E0689.
- I made one array-to-slice coercion explicit.
- The scene builders and readback are the 0068 review helper's code, which
  compiled, against crates unchanged since then.

## Changed paths

- `handoffs/0072-motion-history-numeric/temporal-result.md` (stage A)
- `handoffs/0072-motion-history-numeric/preparation-result.md` (this file)
- `handoffs/0072-motion-history-numeric/visual/plan.json`
- `handoffs/0072-motion-history-numeric/visual/review/temporal_visual_review.rs`
- `handoffs/0072-motion-history-numeric/visual/tools/{common,lease,build,gpu,analyze,selfcheck,test_lifecycle}.py`
- `handoffs/0072-motion-history-numeric/visual/preparation/{selfcheck,lifecycle}.json`

`result.md`, `preflight-result.md`, the brief and the runners are unchanged;
root owns the brief and runners.

## Limitations and open items

- **Uncompiled fixture.** It was never compiled, because no build was
  permitted.
  - A compile error would surface at `build.py review`. That step latches stop
    after the regression suites, which build and run first.
  - The fix would then need a new visual tree and a new grant.
- **No visual evidence yet.** There are no screenshots, pixels, frames or
  appearance judgments. The visual assessment happens only after a grant.
- **Timing check is an inference.** "Models grows" relies on four interleaved
  samples per side. It concerns pass coverage and is not a performance
  measurement.
- **Single reference host.** It will be the local Metal host. There is no phone,
  shipping-profile or other-backend evidence.
- **Root's decision.** Root chooses the Cargo target, the prior pins and whether
  the regression run should also stand in for any further numeric execution.
