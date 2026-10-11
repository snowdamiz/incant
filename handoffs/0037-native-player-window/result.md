# 0037 — Native player window acceptance: result

**Status: acceptance cases 1–5 pass on this Mac for the frozen executable, with two actionable defects and two minor observations.**

- **Defect D1 (moderate).** The window's own green full-screen control ends the session with `Error: SurfaceSize`, exit status 1. This was a labelled probe outside the brief's in-range resize instructions.
- **Defect D2 (minor).** A resize while the window is unfocused shows a non-uniformly stretched stale frame until focus returns.
- **Not claimed:** any phase gate, performance or frame-budget result, mobile, installer, signing, full game release, audible speaker verification or pixel-placement completion.

**Native interaction finished at 2026-10-11T01:58:46Z.** All five game processes I launched have exited (PIDs 82810, 85986, 93832, 99077, 99325). I never activated, closed or manipulated an Incant editor window or the older Incant process (formerly PID 64681), which was not running. Finder was the only other app made frontmost, and only for focus-loss cases. The benchmark can resume.

## Model, effort and packet

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`) in Claude Code / Claude Agent SDK.
- **Effort:** thinking effort **max**.
- **How verified:** both values are reported by this session's own configuration. The ACP-side verification happened before the prompt, as the packet states; I cannot observe the ACP transport from inside the session. Nothing was substituted.
- **Packet revision acknowledged:** `1b1260a`, "Resume priority: foreground ownership". It is applied as follows:
  - Every operation targets the exact PID I launched. `tools/gamewin.swift` refuses any PID whose executable is not `artifacts/player/incant_game_player`.
  - Keyboard input is posted only to that PID (`CGEventPostToPid`).
  - Windows are matched by that PID plus the title.
  - Nothing is terminated by name. Every process ended through its own close button, its own F failure, the full-screen probe's failure, or the probe's automatic exit.
- **Earlier interrupted attempt:** it only verified the hash, read the docs and listed tools. It created no files and launched nothing.

## Binary, fixture and host

- **Executable:** `artifacts/player/incant_game_player`, Mach-O arm64, 8,104,464 bytes.
  - SHA-256 `6b6a53a1ee86fabc9b2c98c7dd4601534bd72521c828bd55c779dbafbe5b3560`.
  - The hash was verified before the first run, and again by `tools/launch.sh` before every launch. The launcher refuses to start on a mismatch.
- **Fixture hashes (match the evidence JSON):**
  - `static.scene` `39e14dce…cda6`
  - `stream.scene` `f6f95cab…f53c`
  - `clips.bin` `72b6718c…155a`
  - `actions.json` `4cc01a6c…a9b4`
  - Nothing under `artifacts/player/` was modified.
- **Host:**
  - Apple M5 Pro (16-core GPU, Metal 4), macOS 26.6 (25G72), Darwin 25.6.0.
  - One built-in Liquid Retina XDR display, 3024×1964 px at 2× (1512×982 pt). The appearance is dark.
- **Tools:**
  - Captures: ScreenCaptureKit single-window screenshots (desktop-independent window filter, no shadow, 2×), via the helper.
  - Window operations: the macOS Accessibility API on the game PID.
  - Focus loss: `NSRunningApplication.activate` on Finder. Setting AX "frontmost" on Finder was silently ignored on this OS (`evidence/run-01-static/focus-method-probe/`).
  - Helper: built with Swift 6.4. Analysis uses the helper plus Python 3.14.8 standard-library scripts.

## Processes

| Run | Scene | PID | Window | UTC | Ended by | Exit | stdout / stderr |
|---|---|---|---|---|---|---|---|
| `run-01-static` | static | 82810 | 29716 | 01:41:35–01:45:25 | close button | **0** | report below / empty |
| `run-02-static` | static | 85986 | 29742 | 01:46:43–01:53:54 | full-screen probe (D1) | **1** | empty / `Error: SurfaceSize` |
| `run-03-stream` | stream | 93832 | 29819 | 01:54:31–01:57:15 | close button | **0** | report below / empty |
| `run-04-static-fail` | static | 99077 | 29862 | 01:57:28–01:57:31 | F key | **1** | empty / `Error: Session(Script(Javascript { phase: "tick", message: "Error: accepted game input failure" }))` |
| `run-05-static-probe` | static + `--lifecycle-probe` | 99325 | 29868 | 01:57:47–01:57:49 | automatic | **0** | probe report below / empty |

**Exact stdout reports:**

- run-01: `{"accepted_frames":5465,"adapter":"Apple M5 Pro","backend":"Metal","dropped_wall_ticks":"8","lifecycle_probe":null,"mode":"native-window","on_screen":true,"surface_reconfigurations":1,"surface_recreations":0,"surface_skipped":0,"tick":5465}`
  - 5,465 ticks over about 230 s of wall time. That fits the long frozen stretches while the window was unfocused.
- run-03: `{"accepted_frames":1766,"adapter":"Apple M5 Pro","backend":"Metal","dropped_wall_ticks":"0","lifecycle_probe":null,"mode":"native-window","on_screen":true,"surface_reconfigurations":1,"surface_recreations":0,"surface_skipped":0,"tick":1766}`
- run-05: `{"accepted_frames":40,"adapter":"Apple M5 Pro","backend":"Metal","dropped_wall_ticks":"0","lifecycle_probe":{"actual_minimize_restore":true,"actual_resize":true,"explicit_surface_recreation":true,"os_loss_injected":false,"suspended_tick":16},"mode":"native-window","on_screen":true,"surface_reconfigurations":3,"surface_recreations":1,"surface_skipped":0,"tick":40}`

The **exit status** column is the player's own status, recorded by `launch.sh` in `exit.txt`. The background task that ran the launcher reports 0 because the wrapper itself succeeds.

## Cases

Unless stated otherwise, positions are measured on 2× captures with the 64 px title bar excluded. The expected scale is 51.96 px per world unit at 480 px content height (60° vertical FOV, camera at z=8), which is H/9.238 for any content height H.

### 1. Startup: pass

`run-01-static/c01-startup.png`, and the same frame 1 s later in `c02-startup-plus1s.png`:

- **Window:** a 640×480 px content area under the title "Incant native game" (frame 320×272 pt).
- **Background:** uniform RGB (20, 21, 25).
- **Model:** one grey right triangle, 52×52 px.
  - Its left edge is at x=164 px, which is the predicted 320 − 3×51.96 = 164.1.
  - Its base lies on the centre row.
- **Not seen:** no clipping, blank regions, stale content or unexpected content.
- **Repeatability:**
  - The two captures are byte-identical, so nothing drifts without input.
  - The same bytes (`0328b8eb…`) appear at startup in run-02 (`c01-start`) and run-04 (`f00-before`), which are separate processes.
  - The streamed scene (`run-03-stream/s01-startup.png`) has identical content. Only the translucent OS title bar differs: rows 0–61, by at most 1/255.
- **Limit:** the earliest capture after a window first appeared was about 0.13 s (run-05 `p-00`), and it already showed the fixture. A blank first present shorter than that would not have been observed.

### 2. D hold/release and focus loss: pass (observation O1)

**Hold and release** (`run-01-static/motion-hold-release.txt`):

- D was held for 928.7 ms, from posted down to posted up.
- Eight captures taken every 100 ms move monotonically, with a fitted slope of 154.7 px/s. The expected 3 u/s × 51.96 px/u is 155.9 px/s.
- The total displacement is 142.7 px against about 144.9 px predicted, which is within one tick.
- After release the frame stays byte-identical for 1.2 s (`c05-released-*`, `c06-stopped`).
- `c04-hold-04.png` shows no trails, tearing or ghosting.
- This is coherence only. No smoothness or performance claim is made.

**Focus loss while D is held** (`run-02-static`, `motion-focus-loss.txt`):

- D went down while the game was focused. The model moves at 152.4 px/s (`c02-held-*`).
- Finder was activated after 338 ms. The frame freezes at centroid 230.0 px for six captures over about 1.05 s (`c03-unfocused-*`).
- The displacement implies the simulation stopped within about one tick of the activation request.
- After refocus, with no new key event, the model shows **one tick** of extra motion (+3 px; one tick at 3 u/s is 2.6 px) and then stays still for about 1.06 s (`c04-refocused-*`). This is O1.
- A new D press moves it again (`c05-new-press-*`), and release stops it (`c06-released-*`). The held control was reset.

**Unplanned focus loss:** during run-01 another app was brought to the front without any action of mine. A D press posted to the unfocused game produced no motion (`run-01-static/c08-held-*`). Those frames are not used for acceptance.

**Audio during focus changes:** this is a capture of the game process's own audio output via ScreenCaptureKit, filtered to that application. It is **not** a speaker observation and not the PCM proof (`run-03-stream/s-audio*.jsonl` and `*-summary.json`, 20 ms buffers).

| Phase | RMS left | RMS right | Notes |
|---|---|---|---|
| Model at x=−3 (focused) | 0.072 | 0.003 | — |
| During a 1.5 s D hold | 0.056 | 0.024 | Pan sweeps with the motion |
| After release (x≈+1.5) | 0.003 | 0.080 | Right-panned |
| Finder frontmost | 0 | 0 | All 87 settled buffers are exactly silent. Silence begins about 0.10 s after the activation request |
| Refocused | 0.003 | 0.079 | Audible about 0.12 s after the request, with the same pan |

### 3. Resize, minimize, restore and focus switching: pass for in-range cases (D2, O2)

**In-range resizes.** All requested while focused, through AX, with captures taken 500 ms later (`run-02-static/r0*.png`):

| Capture | Content (px) | Triangle (px) | Expected H/9.238 | Base row / H/2 | x of left edge (world) |
|---|---|---|---|---|---|
| `r01-centered` | 640×480 | 51×51 | 52.0 | 240 / 240 | −0.500 |
| `r02-req-960x540` | 960×540 | 58×58 | 58.5 | 270 / 270 | −0.496 |
| `r03-req-1920x1080` | 1920×1080 | 116×116 | 116.9 | 540 / 540 | −0.496 |
| `r04-req-800x600` | 800×600 | 64×64 | 65.0 | 300 / 300 | −0.493 |
| `r05-req-320x1080` | 320×1080 | 116×116 | 116.9 | 540 / 540 | −0.496 |
| `r06-req-1920x200` | 1920×200 | 21×21 | 21.7 | 100 / 100 | −0.508 |
| `r07-req-240x180` | 240×180 | 19×19 | 19.5 | 90 / 90 | −0.513 |
| `r08-req-16x16` | 174×16 | 1 px | 1.7 | 8 / 8 | — |
| `r09-req-640x480` | 640×480 | 51×51 | 52.0 | 240 / 240 | −0.500 |

- **Every size re-rendered.** The size is correct, the triangle stays square (no stretch), the base sits on the centre row, and the world position does not change. The content contains only the background plus triangle colours.
- **The 16×16 px request:** AppKit clamped the frame width to 87 pt, so the content was 174×16 px. That is still inside the supported range. The fixture is naturally invisible at that size (one pixel), and the title bar shows only the traffic lights (`r08`). This is not corruption.

**Rapid resizes:**

- 48 AX size changes about 16 ms apart, growing and then shrinking.
- Transient captures taken mid-burst at 960×640, 1280×800 and 1600×960 px already show correct, unstretched geometry (`q-transient-*`).
- The settled 1600×960 and 640×480 captures are correct (`q01`, `q02`).

**Minimize and restore** (`m0*`):

- When minimized, the window is off-screen and the AX `minimized` attribute is true.
- The following frames are byte-identical to one another:
  - `m01-restored-150ms` and `m02-restored-650ms`. Their names give the planned sleeps. The AX state query blocked during the restore animation (see O2), so they were actually taken about 0.60 s and 1.19 s after the restore request.
  - The frame after refocus.
  - The frame after three quick cycles.
- Compared with the pre-minimize frame, they differ only in the OS-drawn title text: x 170–411, y 21–46, at most 1/255.

**Minimize while D is held:**

- The model moved only for the time D was held before minimizing: 23.3 px for 152 ms in run-02 `h0*`, and 31.3 px for 207 ms in run-03 `s04`/`s05`.
- After restore and refocus it stays still until a new key event.

**Focus switching:**

- Three Finder ↔ game round trips leave the content unchanged (`f01-after-3-switches`).

**Lifecycle probe frames** (`run-05-static-probe/p-*`, a fixed-size capture canvas):

- `p-01` and `p-02`: the probe's resize to 512×320 px is rendered correctly. The triangle is 34 px, against 320/9.238 = 34.6.
- `p-03` to `p-12`: the OS genie animation, carrying the last frame intact.
- `p-13` to `p-16`: the frames after restore and the explicit recreation are byte-identical to `p-01` (`bcbd3cbd…`).

**D2: resize while unfocused leaves a stretched stale frame.**

- **Steps:**
  - Finder was frontmost while the game window was resized to 960×540 px (`u01-unfocused-req-960x540.png`).
  - The window then showed the previous 640×480 frame scaled to fill it: the triangle measured 78×58 px instead of 58×58, with 55 resampled colours.
  - At 400×300 the frame was uniformly downscaled but still stale (`u02`).
  - The first capture after refocus is crisp and correct: 32×32 px, 4 colours (`u03`).
- **Cause:** while unfocused the host intentionally neither ticks nor presents, so the layer scales the last drawable.
- **Impact:** resumed content is current, so this is not lifecycle corruption. But macOS lets users and the window manager resize background windows, and the distortion is visible until focus returns.
- **Possible directions (Astra's decision, within the no-new-rendering-path bound):**
  - document it as a limitation;
  - choose layer contents gravity that avoids non-uniform scaling;
  - or allow one non-simulating re-present of committed state.

### 4. Normal close and F failure: pass

**Close button** (pressed through AX on the game window; two separate runs):

- run-01 (static) and run-03 (stream) each exited with status 0, empty stderr and exactly one JSON report on stdout.
- The process and its windows were gone by the 2 s check.

**F key** (run-04):

- One F press posted to the focused game. The process exited about 80 ms later with status 1.
- stderr was exactly `Error: Session(Script(Javascript { phase: "tick", message: "Error: accepted game input failure" }))`. There was no stdout report and no live session or window afterwards.
- I make no claim that submitted GPU work or device audio was or could be rolled back.

### 5. Numerical lifecycle probe: pass

- run-05 exited 0. Its report matches Astra's static record field for field, except `dropped_wall_ticks`: 0 here, 5 there, which is timing-dependent and not a gate.
- The frames above line up with its steps: resize at tick 8, minimize at tick 16 (where the probe suspended), restore after 250 ms, and recreation at tick 24.
- The report confirms explicit recreation of a real surface. **No OS-generated surface loss was injected or observed.**

## D1: the window's full-screen control ends the session

**Evidence** (`run-02-static`, last plan in `timeline.jsonl`, capture `fs00-before.png`):

- **Button state:** the window's zoom button has subrole `AXFullScreenButton` and is enabled. `AXFullScreen` is settable.
- **Action:** I set `AXFullScreen=true` through AX. That is the same toggle as clicking the green button; I did not click a pointer.
- **Result:**
  - Within about 2.5 s, PID 85986 exited with status 1.
  - stderr was `Error: SurfaceSize` and stdout was empty.
  - macOS returned to the previous Space by itself.

**Cause** (read-only, from the committed source; the sources' SHA-256 values match the evidence JSON):

- The window is built with `with_max_inner_size(1920×1080 px)`, but full screen stays enabled.
- On this 3024×1964 px display the full-screen inner size exceeds the bound.
- `SurfaceState::reserve` returns `Error::SurfaceSize`, which is terminal.

**Impact:**

- One ordinary click on the game's own title bar terminates the session with a failure status.
- Any other OS-driven size above the bound could fail the same way, for example a higher-scale display. That was **not** tested, because there is one display.

**Possible directions (Astra's decision):**

- disable full-screen/zoom for this bounded window;
- or treat an out-of-range size as deferred or clamped rather than terminal, while keeping the 16–1920 × 16–1080 surface bounds.

## Minor observations

**O1: one tick of residual motion on refocus.**

- **Symptom:** after a focus loss with D held, the first frame after refocus is one tick (2.6–3 px) beyond the frozen frame, even though the held control is reset. Nothing moves after that.
- **Likely causes** (pixels cannot tell them apart):
  - The script writes `velocity = right.active ? 3 : 0` each tick, so the first resumed tick may integrate the velocity from before the freeze before the script zeroes it.
  - Or presentation lags the simulation by one tick.
- **Severity:** cosmetic. The same one-tick latency applies to an ordinary key release.

**O2: about 0.65 s of audio tail on minimize.**

- **Symptom:** in three measurements (`s-audio`, `s-audio-min`, `s-audio-min2`), app audio continued for 0.64–0.68 s after the minimize request and then dropped to exact silence. On focus loss, silence came after about 0.10 s.
- **Why the simulation still froze:** an AX query sent immediately after `minimize` took 577 ms to return, and one after `restore` took 544 ms. Normal is 15–20 ms. So the app's main thread is unavailable for the whole (de)miniaturize animation. No ticks run, and the world stays frozen, as the minimize-with-D results show. The audio thread keeps playing until the loop resumes and suspends it.
- **Assessment:** this looks platform-bound. A short audible tail may be worth documenting.

## Not tested or unavailable

- **Live pointer edge-drag resize.** A drag posted only to the game PID did not resize the window, because macOS handles edge resizing outside the app's event stream (`l01-after-drag`: content unchanged). I did not post global pointer events, because another app repeatedly came to the front and might have received them.
- **Other window operations:** zoom (Option-click), tiling, display or scale changes, multiple displays and Stage Manager.
- **An actual OS-generated surface `Lost`/`Outdated`/`Timeout` event.**
- **Audible speaker output.** I did not listen. The app-audio capture above is neither the speaker nor the PCM proof.
- **Other platforms:** Linux, Windows, mobile and phones. There is also no performance or frame-budget measurement, and no installer or signing work.
- **Limits of the evidence:**
  - Captures are the window's composited buffer, not physical scanout.
  - Colour values are as captured, so no colour-exact claim is made.
  - Capture timestamps carry 30–90 ms of capture latency.

## Changed paths

- `handoffs/0037-native-player-window/result.md` (this file).
- `handoffs/0037-native-player-window/tools/`:
  - `gamewin.swift`: PID-guarded window, input, capture, app-audio, analysis and diff helper.
  - `launch.sh`: hash-checked launcher that records PID, stdout, stderr and exit status.
  - `audio_summary.py`
  - `motion_fit.py`
- `handoffs/0037-native-player-window/evidence/`:
  - 131 PNG captures, per-run `timeline.jsonl`, `analysis.jsonl`, stdout/stderr/pid/exit files, audio logs and summaries, and motion fits.
  - `SHA256SUMS` covers every evidence file.
  - Capture paths inside the timelines are repo-relative.
- **Uncommitted:** the compiled helper, which lives in `handoffs/0037-native-player-window/artifacts/bin/` (ignored by `**/artifacts/`).
- **Not changed:** production Rust, shaders, styling, build settings, `artifacts/player/`, `brief.md`, and every other handoff.

### Key captures (SHA-256)

| Capture | SHA-256 |
|---|---|
| `evidence/run-01-static/c01-startup.png` | `0328b8ebb4008fdfabb9ba2ce44b946af83bad803f3d8e92aaac9184de93e7c6` |
| `evidence/run-01-static/c04-hold-04.png` | `c4b0c048a9d9d67cde957f8d56c841999921ec08f1751f6d37ac6906435a8094` |
| `evidence/run-01-static/c06-stopped.png` | `52a13f1406d70b3ae2b77f592b32f8299d665308ec6be7c8e486892b55ecf5f5` |
| `evidence/run-02-static/c03-unfocused-00.png` | `4710dfbf786b118a73fc860ef16dfcdb12bae0e22404a43113d5cbaea48d5f42` |
| `evidence/run-02-static/c04-refocused-00.png` | `9e53a0651da93d7c994f1dd155b3eee9f1b8ee323dbb66c462d3de6ee946085a` |
| `evidence/run-02-static/r03-req-1920x1080.png` | `281ed15318961999c3026cda81be768d7a3af2e2a7449e464aaa217b80b10710` |
| `evidence/run-02-static/r05-req-320x1080.png` | `0c875a6326e97c3916daa4b42c8df0d94d3bd1bb3a8a1797402fac8de4817b2e` |
| `evidence/run-02-static/r06-req-1920x200.png` | `bb300003976840b29c916b40acfe67c2e1faa9e1ed50383d4de1887418077542` |
| `evidence/run-02-static/r08-req-16x16.png` | `1a0c5884528eda9d972994a1e29f7c5a41e1a7367b94ab124c4661274eedb534` |
| `evidence/run-02-static/u01-unfocused-req-960x540.png` (D2) | `2ef5358ff3f640245ba90d47354757d7280ca8a4b838571d97f9140eee0d19dd` |
| `evidence/run-02-static/u03-refocused-400x300.png` | `2139f654479ac1ebe6ca5ea2f4c2bf6c32ba08b396192d2c227683b2d7699af7` |
| `evidence/run-02-static/m04-after-3-cycles.png` | `0b4fe52ab6c00e69a0c323a035ab051870cb0a47ad22a2b754d5329eb71371d7` |
| `evidence/run-02-static/q01-settled-big.png` | `55af63fdcb2489101e4858079caa74cd6a2419c16c3da97d1dac2ff80f0c2bad` |
| `evidence/run-02-static/fs00-before.png` (D1, before the probe) | `19fd00c5facbfa21d7f159776c62c285ebfe15801d94c139aa385fe506f54711` |
| `evidence/run-03-stream/s01-startup.png` | `05661267426771446fee9d0632c81a6dd26787340a6bb6e18c29945968d35f73` |
| `evidence/run-04-static-fail/f00-before.png` | `0328b8ebb4008fdfabb9ba2ce44b946af83bad803f3d8e92aaac9184de93e7c6` |
| `evidence/run-05-static-probe/p-04.png` (genie) | `9d16948a442ef851f7feead7c7e4aa550c323ebbbaf0e65eb9d66ead3ab64750` |
| `evidence/run-05-static-probe/p-16.png` | `bcbd3cbd46187283b3ba6f782aebcbbdebbe9f28b0f864fe59a83cc80b08888b` |

## Commands

From the worktree root:

```sh
shasum -a 256 artifacts/player/incant_game_player   # 6b6a53a1…3560
swiftc -O -parse-as-library -o handoffs/0037-native-player-window/artifacts/bin/gamewin \
  handoffs/0037-native-player-window/tools/gamewin.swift
export GAME_BINARY="$PWD/artifacts/player/incant_game_player"
handoffs/0037-native-player-window/artifacts/bin/gamewin perms        # AX, screen capture, post-event: all true
handoffs/0037-native-player-window/tools/launch.sh handoffs/0037-native-player-window/evidence/run-01-static static.scene
handoffs/0037-native-player-window/artifacts/bin/gamewin run PID OUTDIR "game-front; sleep 300; require-focus; cap NAME; ..."
handoffs/0037-native-player-window/tools/launch.sh handoffs/0037-native-player-window/evidence/run-05-static-probe static.scene --lifecycle-probe
handoffs/0037-native-player-window/artifacts/bin/gamewin analyze --top 64 RUN/*.png > RUN/analysis.jsonl
python3 -I handoffs/0037-native-player-window/tools/motion_fit.py RUN FIT_PREFIX [PREFIX ...]
python3 -I handoffs/0037-native-player-window/tools/audio_summary.py handoffs/0037-native-player-window/evidence/run-03-stream s-audio
```

Every exact step plan, with timestamps and results, is recorded in the `begin` entries of each `timeline.jsonl`.

## Open questions

1. **D1:** should the bounded window disable full screen/zoom, or should out-of-range sizes defer or clamp instead of failing?
2. **D2:** is a stretched stale frame acceptable for background resizes, or should contents gravity or a non-simulating re-present be considered within the existing bounds?
3. **O1 and O2:** accept them as normal one-tick input latency and a platform animation tail, or document them in `docs/spikes/native-window-player.md`?
