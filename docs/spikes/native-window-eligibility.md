# Pearl Cove native-window qualification

This packet combines the eligibility plumbing, CPU qualification and actual
win-window proof while preserving the earlier failures. Pearl Cove uses its existing authored tile/sprite,
physics, HUD and recorded physical keyboard-event semantics. No art, gameplay,
input policy, focus rule, route padding or deadline was changed for this proof.

The first main300-based shipping player, `b21a15ff`, passed the existing 781-step
win, 201-step urchin loss and 221-step shore/cliff CPU routes, with eleven total
observation checks. Two actual win-window attempts then hit their unchanged
90-second child deadlines. One included scoped root CUA interaction and four
actual tool images; the other had no CUA interaction. Both were reaped, with no
final report. The first images established crisp authored tiles/sprites/HUD and
a visible player position change, not route completion or sustained focus.

A fresh public shipping export from `ed16a31f` added the default-off bounded
eligibility observer and included the already reviewed observer/audio source
composition. Its player is `4a43581a`; all ten runtime-data files match the old
package exactly. The new package independently passed all three CPU routes.
Every logical report and all 7,699,200 bytes of PCM match the historical package.
This is correctness parity, not a timing comparison or a single-change causal
experiment. The exporter removed its own private target before the inherent
one-tick CPU startup. No existing compiler target was reused for shipping.

The first diagnostic win-window's native child completed normally: Metal on
Apple M5 Pro, 781 host steps/ticks, 782 accepted frames, one presentation refresh,
four exact CPU-matching observations, and the completed recorded-input hash.
The runner still failed because it incorrectly required three native widgets.
Its original refusal, raw report, empty stderr and completed eligibility log are
retained without rewriting the outcome.

The source contract explains the mismatch precisely. CPU reporting counts the
three authored logical nodes: Root, STATUS and PROMPT. Native `UiInput` excludes
Root/Container and includes the two text-bearing labels, including the empty
prompt. The tool-only successor `1bee623b` requires exactly two native widgets
with a strict integer type, keeps the CPU expectation at three, and retains the
frame, replay, observation and positive integer glyph-submission assertions.
Eleven Python controls pass, including missing/extra/root-count and malformed
widget/glyph refusals. The retained raw report passes the repaired oracle in an
offline replay; that replay is not a fresh native execution.

The first diagnostic log has 16 records / 8,145 bytes. It observed initial
unfocused/suspended tick zero at 1.079 seconds, then one focus-gained event by
1.333 seconds and tick 11. Subsequent snapshots remained focused and resumed,
with supported 640×480 size and ready surface reservations, through the final
781-step record at 14.152 seconds. There were no focus-lost events, surface
deferrals/skips or truncation. Forty-nine rapid observations were coalesced, so
the log does not preserve every event's fine ordering. Root performed no CUA
interaction during this child; activation and progress were autonomous. Tao's
native focus query does not independently establish NSApplication foreground.

The earlier timeouts remain historical failures. The fresh binary/source
composition and this successful eligibility trajectory do not identify a sole
cause for those failures. No auto-focus or background simulation was added.

The fresh repaired-runner window passed: native child and runner both exited
zero and were reaped. It completed 781 host steps/ticks and 782 accepted frames,
with exact same-package CPU observations and the original recorded-input hash.
The log retained 49 records / 26,237 bytes. Focus was lost at step 772; snapshots
showed the world remaining at 772 while normally suspended. Root's normal
Finder Open restored cached eligibility at 45.473 seconds, and the remaining
nine steps completed by the final 45.609-second record. One surface deferral and
one skipped surface attempt remain in the report. These are diagnostic elapsed
times with an intentional foreground interruption, not performance measurements.

Root captured three actual 640×512 tool images of the live owned window. They
show readable progress HUD, player positions at different locations, and the
sailing-home outcome with the player at the boat/dock and collected pickups
absent. Root's limited review found crisp intentional pixel-art edges, readable
HUD separation and no observed holes, missing textures or obvious tile seams
in those three stills. This is not continuous-motion or audible-quality review.
The tool images remain in the root conversation, with their receipt in this
packet; no disk PNGs are invented. The 201/221 loss and shore/cliff routes remain
CPU-qualified only; they were not run in a native window here.

Source `ed16a31f` also passed 90 ordinary native-window library tests and 113
presentation/profile library tests, plus both strict all-target Clippy checks.
Those configurations overlap and are not 203 distinct controls. Each executed
the eight observer controls, including exclusive admission, finite output,
actual file observations and a child exiting without Rust Drop. The earlier
data-only oracle corrections and first export supervisor's merged-stderr failure
remain in the archive; neither compiler success nor later success rewrites them.

The default-off trusted CLI `--window-state-log FILE` requires `--stop-after
1..36000`, refuses lifecycle-probe mode and exclusively creates its destination
before devices. A scene/package cannot choose the path. Existing files and final
symlinks are refused; parent storage is trusted. Enabled snapshots add native
queries, serialization and filesystem I/O, coalesce within 250 ms, otherwise use
a one-second heartbeat, and stop at 512 records / 512 KiB. Completed writes
survive process timeout; this is not a power-loss durability guarantee. Default
operation adds no native query, world read, serialization or file I/O.

The [manifest](evidence/native-window-eligibility/manifest.json) binds the raw
receipts, source copies, commands, retained binary/PCM hashes and root image
scope. Replay performs no player, build, device or authoring operation:

```sh
python3 docs/spikes/evidence/native-window-eligibility/verify.py
python3 docs/spikes/evidence/native-window-eligibility/verify.py \
  --originals --binaries --git-source HEAD --source-root .
```

Counts and native submission receipts do not prove exact final text or rendered
appearance. Actual root tool images, if captured while the owned child is live,
remain separately scoped. No FPS, performance budget, glitch-free audio, other
device or phase-gate claim follows. Recorded physical input does not prove live
keyboard handling, and this slice adds no localization/RTL/CJK qualification.
