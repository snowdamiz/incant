# 0059 result: checkpoint 12 UI UNORM mismatch (Linux llvmpipe and Windows WARP)

**Status: root cause established and reproduced exactly; reference corrected;
local Metal verification done. Hosted Linux and Windows confirmation is
pending (Astra's integration owner).** This claims no Game UI, device or
phase gate.

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`) with thinking
  effort **max**, in the ACP handoff session started by
  `tools/handoff/main.py run 0059-linux-ui-reference`. Root holds the ACP
  settings verification. No other model, subagent or reviewer was used.
- **Brief applied:** sha256 `15b2f7f6…f2b7`, which is the routing commit
  `f40355ed…3740` plus root's priority revision.
- **Priority revision acknowledged:** the actual Windows DX12 failure on WARP
  is used as independent evidence. The bound follows Vulkan and Direct3D
  rules, not Mesa alone. Windows is listed in pending hosted verification, and
  nothing claims Windows or Linux acceptance from Metal. Scope stayed on the
  reference contract.
- **Root files:** `brief.md` and both `evidence/checkpoint12-windows-*` files
  are left unchanged and uncommitted.

## Root cause

**The independent reference was wrong; the implementation was not.**

The reference modelled a UNORM target as a float blend in which only the
stored code is rounded (`ENCODE` 0.67, destination read back exactly). Both
APIs allow more latitude:

- **Vulkan:** "basic blend operations are performed with a precision and
  dynamic range no lower than that used to represent destination components".
- **D3D11.3 §17.5:** for fixed-point targets, "blending operations may be
  performed at equal or more … precision/range than the output format".
- **D3D11.3 §3.2.3.6:** float→UNORM is round-half-up "with a permitted
  tolerance of 0.6f ULP".

Both CI CPU rasterizers blend `Rgba8Unorm` in 8-bit integers. From the
llvmpipe source of the exact CI package (Ubuntu
`mesa 25.2.8-0ubuntu0.24.04.4`, tarballs checked against the signed `.dsc`;
no Ubuntu patch touches llvmpipe, gallivm or lavapipe):

- **Blend type:** `lp_blend_type_from_format_desc` makes UNORM8 targets blend
  as 8-bit unsigned norm. sRGB formats blend in float32, which is why every
  sRGB case passed.
- **Conversion:** `lp_bld_conv.c` converts every output channel, including the
  alpha used for the factor, as `cvtps2dq(min(1,x)·255)`, round half to even,
  with saturating packs.
- **Blend:** `lp_bld_blend.c` (ONE / INV_SRC_ALPHA is not a "complementary"
  pair, so the generic path runs) computes `uadd.sat(src8, mul_norm(dst8, ~a8))`.
  `lp_build_mul_norm` is `(t + (t >> 8) + 128) >> 8`. Checked exhaustively, it
  is correctly rounded except for 24 of the 65,536 8-bit products, which round
  down by 128/255 code.

Each of these steps rounds by about half a code at most, so one blend can sit
about 1.5 codes from the float blend. The single 2-code refusal comes from two partial blends
at pixel (33,59), red channel, over background 231:

1. Label shadow (opacity 150, coverage 8/255): A = 4.71 → 5 and the product
   226.47 → 226, so it stores 226 (the float blend gives 226.74).
2. Text (236, coverage 48/255): S = 44.42 → 44 and the product 183.46 → 183,
   so it stores 227.

Exact arithmetic gives 229. The float-only interval was [229, 229]; the
corrected one is [227, 229].

The shader cannot choose output-merger precision. Its output already follows
the contract: Metal's UNORM image equals the exact model bit for bit.

## Fix (commit `5186268876a7f651c462921eca143b34fb4e09cc`)

**`ui_draw_tests.rs`, `Model::Portable`:** a UNORM channel now admits either
the float blend (unchanged) or a blend at the target's 8-bit precision:

- source and alpha codes within `CONVERT` = 0.6 code (the D3D tolerance; it
  includes Vulkan's recommended and llvmpipe's actual nearest rounding);
- the destination product within `PRODUCT` = 128/255 code (llvmpipe's
  measured worst case);
- a saturating sum.

That is at most 0.6 + 0.6 + 128/255 ≈ 1.71 codes from exact per blend. It is
exact for opaque fragments, and discarded fragments still store nothing.
Truncating or coarser blend arithmetic is documented as outside the reference.

**`Model::Llvmpipe`:** now emulates UNORM8 targets from the 25.2.8 source.

**CPU test:** it now also runs the emulation at 1x UNORM and prints the GPU
test's statistics. It gains a blending-space negative control (linear light
on UNORM, encoded values on sRGB) next to the existing shift, opacity and clip
controls.

**Unchanged:** the shader, the renderer and the zero-coverage discard. The
`ui/mod.rs` change is a contract comment only, and the spike doc evidence
paragraph is updated.

## Evidence (all in `evidence/`; raw logs in `~/.codex/incant-evidence/0059/logs`)

| Item | Result |
| --- | --- |
| Receipt source hashes before diagnosis | all five matched; no renderer change main150→main151 |
| Linux log | raw sha256 `fb5e3503…37eb` (gz `9759d8ae…083a`); adapter printed `llvmpipe (LLVM 20.1.2, 256 bits)`, Mesa 25.2.8-0ubuntu0.24.04.4, Vulkan |
| Windows log | raw sha256 `cec7da5a…1ebb` (gz `e6549406…651c`); adapter printed `Microsoft Basic Render Driver`, 10.0.26100.33438, Dx12 |
| CI 1x UNORM line (both backends) | `outside 245 by 2, widest 2 (0 channels > 2); vs exact worst 2, >1: 5, changed 4248` |
| CPU llvmpipe emulation vs the old reference | identical line, all five statistics (`checks/diag-unorm-emulation-vs-base-reference.txt`); the 1x sRGB emulation line also equals CI's |
| Reference mutated back to float-only | CPU test fails with CI's assertion verbatim: `Admission { outside: 245, excess: 2.0, widest: 2.0, wide: 0 }` |
| Corrected reference, 1x UNORM | emulation admitted; widest span still 2 (0 channels > 2); 508 of 43,200 channels gain codes, each pixel with a partial-alpha blend; 3,014 opaque-only and 10,150 untouched pixels keep exactly one code |

**Fingerprint replay.** `tools/unorm_fingerprint.py` replays the reference's
own blend trace (`unorm-1x-trace.txt.gz`, written by `tools/diag-trace.patch`)
under each candidate arithmetic. It reproduces the Rust references and the
emulation bit for bit.

| Arithmetic | vs float-only reference | Corrected reference |
| --- | --- | --- |
| llvmpipe (cvtps2dq, Blinn) | **= CI** | admitted |
| 8-bit, nearest-even, exact product | **= CI** | admitted |
| 8-bit, D3D half-up, exact product | **= CI** | admitted |
| 8-bit truncation | outside 2739 by 4 | 2533 outside |
| nearest-even, `a·(b+1)>>8` | outside 1338 | 1210 outside |
| nearest-even, `(t+(t>>8))>>8` | outside 1707 | 1592 outside |
| float blend, nearest store (Metal-like) | outside 0, worst 0 | admitted |
| float blend, truncated store | outside 1635 | 1428 outside |

So CI's shared fingerprint picks out nearest-rounding 8-bit blending, not a
Mesa quirk. WARP's printed statistics fit correctly rounded 8-bit blending.
Its images were not available.

**Local adapter: Apple M5 Pro, Metal (IntegratedGpu), as printed by every UI
GPU test.**

| Run | Result |
| --- | --- |
| Before (base) | 6/6 UI draw tests pass; 1x UNORM `outside 0 … vs exact worst 0, >1: 0, changed 4246` |
| After (fix) | 6/6 pass, same statistics; all 8 test images byte-identical before and after (hashes in the JSON) |
| `ui::` (CPU geometry and GPU draw) | 14 passed |
| incant_render CPU lib | 59 passed, 32 ignored |
| incant_render GPU subset, as CI runs it (`--ignored`) | 32 passed (on llvmpipe and WARP: 31 + 1 failed) |

**Negative controls after the fix, 1x UNORM** (the sRGB scales are unchanged
from before):

| Variant | Rejected by |
| --- | --- |
| Other blend space | 3,303 channels, excess 67 |
| Text 1 px right | 3,111 channels, excess 223 |
| 90 % text opacity | 2,219 channels, excess 23 |
| Missing ancestor clip | 146 channels, excess 230 |

**GPU mutations on Metal** (`evidence/mutations.json`): 7 of 7 caught. The
unmutated suite passes before and after, and every file was restored by hash.

| Mutation | Result |
| --- | --- |
| UNORM blends decoded colours | outside 12,045 by 73 |
| UNORM colour 2 % dark | outside 10,367 by 5 |
| UNORM colour **1 %** dark | outside 5,390 by 3 |
| UNORM opacity 97 % | outside 10,604 by 7 |
| Glyph pen 1 px right | 4 tests fail |
| No clip intersection | 4 tests fail |
| Reference float-only | the CI failure above |

**Checks:**

- `cargo fmt --all --check`: clean.
- `clippy -p incant_render --all-targets --profile ci -D warnings`: clean.
- `check -p incant_render --lib --target wasm32-unknown-unknown` (the nightly
  command) and the same with `--profile ci`: clean.

**Performance:** no shader, renderer, allocation or runtime change, and the
Metal output is byte-identical. The UI draw tests took 0.29 s before and
0.31 s after (ci profile; ci builds are not performance evidence).

## Changed paths

- Commit `5186268`:
  - `crates/incant_render/src/ui_draw_tests.rs`
  - `crates/incant_render/src/ui/mod.rs` (comment)
  - `handoffs/0059-linux-ui-reference/tools/{cargo.sh,diag-trace.patch,mutate.py,unorm_fingerprint.py}`
- This commit:
  - `docs/spikes/native-ui-draw.md`
  - `handoffs/0059-linux-ui-reference/result.md`
  - `tools/{mutate.py (parser),unorm_fingerprint.py (exactness check),retain.py}`
  - `evidence/{checkpoint12-ui-unorm-precision.json,mutations.json,unorm-1x-trace.txt.gz,lease.json,release.json,commands.jsonl}`
  - `evidence/checks/*.txt` with `manifest.json` (raw and committed hashes)

## Commands

Every cargo run went through `tools/cargo.sh <label> …`, which sets
`CARGO_TARGET_DIR=~/.codex/incant-native-target` and uses rustc 1.99.0 via
`rust-toolchain.toml`. Each run is in `evidence/commands.jsonl`, 23 entries,
with argv, exit status and log hash.

- `test -p incant_render --profile ci --locked -j2 --lib ui::draw_tests -- --include-ignored --test-threads=1 --nocapture`
  - before: exit 0
  - after: exit 0
  - 9 mutation runs
- The same command with the filter `ui::draw_tests::reference_admits`:
  - against the base reference: exit 101, the CI reproduction
  - after the fix: exit 0
- `ui::` with `--include-ignored`, the crate `--lib` CPU set, and the crate
  `--lib -- --ignored` GPU subset: exit 0.
- `clippy` and both `wasm32` checks as listed under Checks: exit 0.
- `cargo fmt --all --check`: exit 0.
- `python3 -I -B tools/unorm_fingerprint.py <trace>` and
  `python3 -I -B tools/mutate.py …`: exit 0.

The first baseline attempt failed before building, because rustc was not on
the runner's PATH (`checks/before-ui-draw-tests-attempt0-no-rustc-on-path.txt`).
After the fix it reran with exit 0. The diagnostic patch was applied for one
run and reverted; the hash was restored to `bc671c0b…` and checked.

## Screenshots

None. No window or screen capture was taken, and none was needed. The
offscreen test outputs, which are not screenshots, are in
`~/.codex/incant-evidence/0059/gpu-{before,after}/*.png`.

## Unavailable and pending

- **Hosted reruns pending:** Linux llvmpipe and Windows WARP runs of the
  corrected reference are not run here. Astra's integration owner runs them.
  No CI change, remote trigger, push or merge was made.
- **No local Linux software Vulkan and no WARP:** the Docker daemon is not
  running and was not started, and there is no x86 VM. The aarch64 Android
  emulator that is running is unrelated and not owned. Metal results do not
  prove Linux or Windows.

## Limitations

- WARP's arithmetic is inferred from its printed statistics only.
- The reference excludes truncating or coarser 8-bit blends, which Vulkan
  permits, and backends below 8 sub-texel bits.
- At 1x every glyph tap lands on a texel centre. UNORM is exercised at 1x
  only, as in the GPU test.

## Processes and lease

Lease `~/.codex/incant-evidence/0059/lease.json` ran from 17:25:52Z to the
release at 17:46:38Z (`released` marker, `release.json`). `tools/cargo.sh`
now refuses with exit 97.

- **Reaped:** no owned process remains. All 23 cargo runs, the mutation driver
  and the Python replays exited.
- **Not owned, left alone:** root's ACP runner and Astra's cargo runs in
  `incant-integration-target`.
- **Target dir:** kept as a warm cache; nothing deleted.

## Open questions

1. **Truncating hosts:** should the reference ever admit truncating UNORM
   blends (Vulkan's "one of the two closest")? I left them out because no CI
   adapter uses them and they would widen every UNORM channel.
2. **UNORM coverage:** the GPU test checks UNORM only at 1x. Adding 2x and
   1.5x UNORM would exercise the fixed-point term with fractional taps; it is
   left out to keep the scope narrow.
