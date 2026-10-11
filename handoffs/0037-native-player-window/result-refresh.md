# 0037 delta — corrected native window (D1, D2): result-refresh

**Status: D1 and D2 are accepted for the limited scope tested below, on this Mac, with the corrected frozen executable.**

- **D1:** the green maximize/full-screen control is disabled and inert, and ordinary close succeeds.
- **D2:** a background resize now shows a fresh render at the correct camera aspect within about 60 ms, during a continuous resize burst too. The world stays frozen until refocus, and D works afterwards.
- **Unresolved:**
  - The window still reports full screen as settable through AX.
  - The out-of-range suspend/recover path could not be exercised. macOS clamps PID-scoped AX size requests to the bound, and I did not force full screen.
- **Not claimed:** any phase gate, mobile, performance or frame budget, installer, signing, phone, full release or pixel-placement completion.

**Native interaction for this delta finished at 2026-10-11T02:25:17Z.**

- Both review PIDs, 41087 and 41919, were launched from `artifacts/player-refresh/` and closed through their own close buttons with exit 0.
- No player process remains.
- No other Incant app or window was touched. Finder was the only other app activated, and only for focus loss.
- The benchmark may resume.

## Model, packet and identities

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`) with thinking **max**, as reported by this session's own configuration.
  - The packet states that ACP reverified both for this resumed session; I cannot observe ACP from inside the session.
  - Nothing was substituted.
- **Packet:** "Priority revision — corrected window delta only" (`e1bf752`, integrated in `93875f2`). Applied as follows:
  - **Immutable originals:** `result.md`, every `evidence/` file and `artifacts/player/` are untouched.
    - `result.md` SHA-256 `23ae3be8…8af` equals its version at `4073f8f`.
    - `evidence/SHA256SUMS` re-verifies all 180 files.
    - `artifacts/player/incant_game_player` is still `6b6a53a1…3560`.
  - **Old helpers unchanged:** the old launcher and helpers are byte-identical to `4073f8f` (`tools/launch.sh` `fc395fa3…`, `tools/gamewin.swift` `b1ffb1fd…`).
  - **Written:** only new helpers, `evidence-refresh/` and this file.
- **Source under review:** `d11c7c4`, "Refresh paused native windows without advancing gameplay". I read its diff only to target the tests.
- **Executable:** `artifacts/player-refresh/incant_game_player`, Mach-O arm64, **8,120,976 bytes**.
  - SHA-256 `ec18e426c67584c4ac762cf72deda6dfc10d9044c4a5ff91a07fc4ad0648edf5`.
  - `tools/launch-refresh.sh` verifies it before every launch.
- **Fixtures:** every fixture file in `artifacts/player-refresh/` is byte-identical to its counterpart in `artifacts/player/`. That includes `static.scene` `39e14dce…`, `clips.bin` `72b6718c…`, `actions.json` `4cc01a6c…`, the audio cache and the model cache.
- **Host:** unchanged from the original review. Apple M5 Pro / Metal, macOS 26.6 (25G72), built-in 3024×1964 px display at 2×.
- **Tools:**
  - The compiled `gamewin` (built from the committed `tools/gamewin.swift`), run with `GAME_BINARY=$PWD/artifacts/player-refresh/incant_game_player`. It refuses any PID whose executable is not that path.
  - New `tools/axprobe_refresh.swift`, a PID-guarded reader for title-bar buttons, the full-screen attribute and the menu, which can also press one button.
  - New `tools/refresh_geometry.py`, which applies the prior reference math.

## Processes

| Run | PID | Window | UTC | Ended by | Exit | stderr |
|---|---|---|---|---|---|---|
| `evidence-refresh/d1-static` | 41087 | 30201 | 02:22:06–02:23:24 | close button | **0** | empty |
| `evidence-refresh/d2-static` | 41919 | 30349 | 02:24:11–02:25:17 | close button | **0** | empty |

**Exact stdout reports:**

- **d1:** `{"accepted_frames":2045,"adapter":"Apple M5 Pro","backend":"Metal","dropped_wall_ticks":"8","lifecycle_probe":null,"mode":"native-window","on_screen":true,"presentation_refreshes":0,"surface_reconfigurations":4,"surface_recreations":0,"surface_skipped":2,"tick":2045}`
  - 2045 = 2045 + 0. No background redraw was pending in this run.
  - `surface_skipped` counts reservations deferred for an unsupported or minimized size, or for a deferred acquisition. The report does not say which; the session continued.
- **d2:** `{"accepted_frames":913,"adapter":"Apple M5 Pro","backend":"Metal","dropped_wall_ticks":"0","lifecycle_probe":null,"mode":"native-window","on_screen":true,"presentation_refreshes":16,"surface_reconfigurations":17,"surface_recreations":0,"surface_skipped":2,"tick":897}`
  - `accepted_frames` 913 = `tick` 897 + `presentation_refreshes` 16.

Geometry below uses the original reference math (`tools/refresh_geometry.py`):

- **Content:** each capture minus its 64 px title bar.
- **Scale:** one world unit is H/9.2376 px.
- **A fresh render:** a square triangle of that size with its base on row H/2, and at most 8 content colours. Fresh frames here have 4–7 colours. The original stale frames had 55 (`u01`) and 12 (`u02`), and the script still classifies those two as not fresh.
- **`world_left`:** the triangle's left edge in world units, so positions compare across window sizes.

## D1: the maximize/full-screen affordance is disabled and inert; close succeeds (accepted, with a residual)

**Before the fix:**

- The green control was enabled (`AXFullScreenButton`).
- Pressing it ended the session with `Error: SurfaceSize` (original `result.md`, D1).

**Now** (`d1-static/buttons-before.json`):

- **AX state:**
  - The control (subrole `AXFullScreenButton`, exposed as both the zoom and the full-screen button) is **`enabled: false`**.
  - Close and minimize stay enabled.
  - `AXFullScreen` is false.
- **Visual** (`a01-startup.png`):
  - The third traffic light is grey while the window is active.
  - A diff against the original startup capture (`evidence/run-01-static/c01-startup.png`) changes 664 px, all inside that button (x 110–137, y 18–45). The game content is byte-identical.
- **Pressing the disabled control** through AX (`press-zoom.json`):
  - AX returned 0, and nothing happened.
  - 1.5 s later the frame is byte-identical (`a02` = `a01`, `9444c0de…`), the window is still 320×272 pt, `AXFullScreen` is still false, and the PID is alive (`buttons-after-press.json`).
- **Menu:** the application exposes no AX menu bar (`menu.txt`), so there is no "Enter Full Screen" or "Zoom" menu item.

**Out-of-bound sizes.** I sent PID-scoped, reversible AX size requests. The OS refused to leave the bound:

| Request | Result | Capture |
|---|---|---|
| Content 2000×1200 px (frame 1000×632 pt) | Clamped to **1920×1080 px** (960×572 pt) | `a03` |
| Content 600×4 px (frame 300×34 pt) | Clamped to **600×16 px** (300×40 pt) | `a05` |

- `a03` is byte-identical to an explicit maximum request (`a04`, `693dfad0…`), and fresh: a 117×117 triangle against an expected 116.9.
- `a05` is fresh. At 16 px tall the fixture shrinks to a single pixel, which is natural at that size.
- Returning to 640×480 (`a06`) is fresh, with the triangle back at x = −3.002.
- Against `a01`, `a06` differs only in the translucent OS title bar: rows 0–61, at most 4/255. The two were taken at different window positions.
- So the fix's "out-of-range OS sizes suspend and recover" path **could not be exercised** with a reversible AX request. Per the packet, I did not force full screen or invent another route.

**Ordinary close:** the close button gave exit 0, empty stderr and the report above.

**Residual:**

- `AXFullScreen` is still **settable** through AX, before and after the press. Programmatic full screen, and possibly the system Globe-F shortcut (untested), may still request a size above the bound.
- The fix routes such sizes into suspension rather than termination, but I did not observe that path.

## D2: a background resize shows a fresh render, the world stays frozen, and D works afterwards (accepted)

**Timeline in `d2-static/timeline.jsonl`:**

- D was pressed while focused at 820 ms.
- Finder was activated at 1032 ms, with D still logically down.
- Every background step below was guarded by `require-unfocus`, with Finder (PID 1608) frontmost.

| Capture | State | Content (px) | Triangle (px) | Expected | Base / H/2 | `world_left` | Colours |
|---|---|---|---|---|---|---|---|
| `b01-focused-640x480` | focused | 640×480 | 52×52 | 52.0 | 240 / 240 | −3.002 | 5 |
| `b02-unfocused-640x480` | Finder front | 640×480 | 51×51 | 52.0 | 240 / 240 | −2.444 | 7 |
| `b03-960x540-immediate`, `-100ms`, `-settled` | Finder front | 960×540 | 58×58 | 58.5 | 270 / 270 | −2.446 | 6 |
| `b04-400x300-immediate`, `-100ms`, `-settled` | Finder front | 400×300 | 32×32 | 32.5 | 150 / 150 | −2.463 | 6 |
| `b05-burst-step10` | Finder front, mid-burst | 800×500 | 54×54 | 54.1 | 250 / 250 | −2.457 | 6 |
| `b05-burst-step20` | Finder front, mid-burst | 1200×700 | 76×76 | 75.8 | 350 / 350 | −2.455 | 7 |
| `b05-burst-step30` = `b06-burst-settled` | Finder front | 1600×900 | 97×97 | 97.4 | 450 / 450 | −2.453 | 7 |
| `b07-refocused-00…03` | refocused | 1600×900 | 97×97 | 97.4 | 450 / 450 | −2.402 | 7 |
| `b08-after-d`, `b09-stopped-00…01`, `b10-before-close` | focused | 1600×900 | 97×97 | 97.4 | 450 / 450 | −1.447 | 5 |

**Fresh render at the correct aspect.**

- At 960×540 and at 400×300, the first capture after the AX resize returned is already a fresh render:
  - at 960×540, the capture started 29 ms and returned 57 ms after the resize;
  - at 400×300, it started 34 ms and returned 63 ms after.
- The 100 ms and settled captures at each size are byte-identical to it (`224f0b19…`, `fb23a8b2…`).
- The original stretched framebuffer (`u01`: 78×58 px, 55 colours) is gone. The unfocused window shows grey traffic lights with a crisp, square triangle.

**Continuous burst.**

- 30 AX resizes, about 28 ms apart (16 ms sleep plus the AX call), took the content from 440×320 to 1600×900 px over about 0.9 s.
- Captures taken immediately after steps 10, 20 and 30, with no settle, are already fresh at the current size.
- The settled frame equals step 30.
- The 50 ms throttle kept presenting during continuous events. The run's 32 unfocused resize requests produced 16 `presentation_refreshes`.

**Frozen until refocus.**

- `world_left` stays between −2.444 and −2.463 across every background refresh. That is within 0.02 units, or at most 0.6 px of rasterisation at 400×300.
- The last tick before focus loss had velocity 3 in effect (D held). A single accidental physics step would have added 0.05 units, which is 2.9 px at 960×540. None appears.
- The pre-freeze motion, −3.002 → −2.444, is consistent with the ~0.21 s that D was held before activation.

**Refocus and D:**

- **Refocus:** the first captures after refocus show +0.051 units, exactly one tick (retained limitation O1). The model then stays still for 4 captures over about 0.48 s.
- **D input:** a 321 ms press moved it +0.955 units, about 19 ticks at 3 u/s.
- **Release:** it stopped, and `b08` = `b09` = `b10` (`00b6539c…`).

## Retained limitations and unresolved issues

- **O1 and O2** from the original review are retained as documented limitations: one tick of input latency, and the audio tail during the macOS minimize animation. Per the packet, they were not re-measured. O1 reappears as expected on refocus.
- **D1 residual:** `AXFullScreen` is still settable; full screen through AX or keyboard was not exercised; and the suspend/recover behaviour for out-of-range sizes has no visual evidence on this host. That evidence would need full screen, a display-scale change or another display, none of which was in scope or available. Astra may decide whether full-screen collection behaviour should also be removed.
- **Not tested:**
  - Live pointer edge-drag resize, as in the original review: posted drags don't resize, and global pointer events were not used.
  - Minimize/restore with the new executable, and the streamed scene. Both are outside this delta.
- **Focus noise from other apps:** the browser and the ChatGPT app took focus intermittently without any action of mine. Guards stopped two D1 plans before any interaction (`require-focus failed`). I reran them, and no frame from those intervals is used.
- **Evidence limits:**
  - Captures are composited window buffers, not scanout.
  - No colour-exact claim is made.
  - Latency figures include 30–60 ms of capture latency and are informal, not budgets.

## Changed paths (this delta only)

**New:**

- `handoffs/0037-native-player-window/result-refresh.md`
- `handoffs/0037-native-player-window/tools/launch-refresh.sh`
- `handoffs/0037-native-player-window/tools/axprobe_refresh.swift`
- `handoffs/0037-native-player-window/tools/refresh_geometry.py`
- `handoffs/0037-native-player-window/evidence-refresh/`:
  - 27 PNG captures;
  - per run: `timeline.jsonl` (repo-relative paths), `analysis.jsonl`, `geometry.jsonl`, and the stdout/stderr/pid/exit/start/end files;
  - D1: AX button, press and menu records;
  - `SHA256SUMS` (49 files; manifest `f878983a…`).

**Compiled helpers (not committed):** `gamewin` and `axprobe` in the ignored `handoffs/0037-native-player-window/artifacts/bin/`.

**Unchanged:** production source, both `artifacts/` player directories, `brief.md`, `result.md`, `evidence/`, the old tools, and every other handoff.

### Key captures (SHA-256)

| Capture | SHA-256 |
|---|---|
| `evidence-refresh/d1-static/a01-startup.png` (disabled control; = `a02`) | `9444c0de2962718362bcc7bd5d99253b70316d19693d002dc9d86a5a7724c5c1` |
| `evidence-refresh/d1-static/a03-req-2000x1200.png` (= `a04`, clamped max) | `693dfad0175126eb46d5f42801b2440907c4c454b59bd46b8c10c3c1c4379c71` |
| `evidence-refresh/d1-static/a05-req-600x4.png` (clamped to 600×16) | `2785c000cc5e2db3b2863b40dcc0b9c88573f164c9280c18f8b1a22efab5aa36` |
| `evidence-refresh/d1-static/a06-back-640x480.png` (= `a07`, = d2 `b01`) | `c72597a6606405127995e094749cfd22f929df10f58a271e140fa44a66946a62` |
| `evidence-refresh/d2-static/b02-unfocused-640x480.png` | `35a347eaef5f8e4a7c23e1a83014fe39a78cb232193208502e47d99abda9915c` |
| `evidence-refresh/d2-static/b03-960x540-immediate.png` (= `-100ms`, `-settled`) | `224f0b199681a7559d6a193d2d09b66f5d5947e223848f4f9c66faa639e3f307` |
| `evidence-refresh/d2-static/b04-400x300-immediate.png` (= `-100ms`, `-settled`) | `fb23a8b274e61b8d9eecd207ee323cc09fa6b7cae96d1e0ce153c8a745ee7187` |
| `evidence-refresh/d2-static/b05-burst-step20.png` | `a43f65c18eb6a92830b95edd4895fd1c1353b1f63e516ce4b1696bacf1509b11` |
| `evidence-refresh/d2-static/b06-burst-settled.png` (= step 30) | `2a79bf544b8ab310d2e778302acb8af52c8e5b8a29ce1d94b8d4e24f99d76c3e` |
| `evidence-refresh/d2-static/b07-refocused-00.png` (= `01`–`03`) | `ddf42c5a739d632868f11477a5cea344be61a1a145e48392ac59883bf68812d2` |
| `evidence-refresh/d2-static/b08-after-d.png` (= `b09`, `b10`) | `00b6539c6ce4918e20606b4625ce9ccadb9239955299704a4ce75d98d302185e` |

## Commands

From the worktree root:

```sh
shasum -a 256 artifacts/player-refresh/incant_game_player       # ec18e426…edf5
swiftc -O -o handoffs/0037-native-player-window/artifacts/bin/axprobe \
  handoffs/0037-native-player-window/tools/axprobe_refresh.swift
export GAME_BINARY="$PWD/artifacts/player-refresh/incant_game_player"
handoffs/0037-native-player-window/tools/launch-refresh.sh handoffs/0037-native-player-window/evidence-refresh/d1-static static.scene
handoffs/0037-native-player-window/artifacts/bin/axprobe buttons PID      # also: menu PID, press PID zoom
handoffs/0037-native-player-window/artifacts/bin/gamewin run PID OUTDIR "game-front; sleep 300; require-focus; size 1000 632; ..."
handoffs/0037-native-player-window/tools/launch-refresh.sh handoffs/0037-native-player-window/evidence-refresh/d2-static static.scene
handoffs/0037-native-player-window/artifacts/bin/gamewin analyze --top 64 RUN/*.png > RUN/analysis.jsonl
python3 -I handoffs/0037-native-player-window/tools/refresh_geometry.py RUN/analysis.jsonl > RUN/geometry.jsonl
```

Each run's `timeline.jsonl` records every exact plan in its `begin` entries.

## Open question

Should the window also drop full-screen collection behaviour, so that `AXFullScreen` is not settable? Or is out-of-range suspension the intended safety net? That suspension is unobserved on this host.
