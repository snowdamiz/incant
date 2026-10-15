# Corrected Arabic native replay and Mac session preflight

The ordinary corrected Arabic shipping package completes two full native Mac
loss/restart/win replays, each matching every same-package CPU observation.
Actual images show readable normal-size Arabic, the loss/restart prompt,
restarted gameplay, and the corrected right-to-left health-pip order. The winning
screen's pixels remain unverified. This is partial locale qualification, not
complete rendering, audio, performance or Phase 1 acceptance.

## Window admission correction

The previous external launcher incorrectly required two macOS lock fields to
exist. On this unlocked Mac, `IOConsoleLocked` explicitly reports false and the
calling user's unique console session reports completed login, while the optional
`CGSSessionScreenIsLocked` field is absent. That absence caused false refusals.

`tools/macos_session.py` now requires the explicit primary unlocked state, exactly
one console session for the current UID, and completed login. Any present
secondary lock field must also explicitly be false. Missing primary state,
malformed fields, another user, conflicting flags or ambiguous console sessions
still refuse. The selected-root plist may be a dictionary or a single-root list.
The locale probe checks this immediately before launching its native window;
CPU-only stages remain unchanged. It invokes no capture or application API.

Eight focused parser controls and the full Python tooling suite pass: 554 tests,
four skips. Independent source review accepted the change. An actual metadata
query and the native replays exercise the previously refused unlocked state.
This is a launcher preflight, not an authentication boundary or a promise that
focus and lock state remain unchanged after dispatch.

## Actual native results

The player retains producing source
`069bbe54dc529511138867dfd4fbe50507a38b6b` and SHA-256
`6e7994630c36cde871a0dea8f2818e93ad2aa9e459d6a2fd848474b71d6835c7`.
It is the [previously exported ordinary package](core-arabic-corrected-export.md),
with unchanged recordings, scripts and runtime data. Later main commits do not
relabel this binary. The native adapter reports Apple M5 Pro; exact machine and
OS metadata are retained in the packet.

| Attempt | Native result | Actual pixels and limitations |
| --- | --- | --- |
| 002 | All 3,056 host steps; generation two ends at simulation tick 2,156; exact CPU observation parity | Loss headline and restart prompt are readable. Restarted gameplay shows one empty health pip on the left and two filled pips on the right. Final capture timed out after the application exited. |
| 003 | The same complete replay and exact CPU observation parity | Undamaged three-pip HUD and gameplay are readable. The process had already exited when the final capture poll ran. |
| 004 | Original 120-second deadline expired after focus loss at host step 13 | Initial HUD is readable. The runtime correctly pauses while unfocused; this attempt does not supply a completed replay or winning pixels. |

The complete runs use 640×480 logical units and 1280×960 physical client size.
The last paused run reported a 640×480 physical size; it is preserved separately.
The final attempt used an event-driven capture of the replay's own log to avoid
missing the short win screen. An unsupported Node module import refused first;
the corrected file-only watcher expired while gameplay was focus-paused. This
does not establish a renderer or file-notification defect.

All three native children returned or were stopped by their original managed
deadline. Cleanup completed, and root independently verified that their owned
PIDs were absent. CUA sessions were reset after use. Earlier locked-desktop
refusals and focus interruptions remain retained.

## Remaining qualification and replay

Arabic win-screen review, CJK's final win image, and the complete normal/narrow
pseudo-localized replay remain open. These static images do not verify motion
reconstruction quality, frame pacing, physical mouse capture or audible output.

The [portable packet](evidence/core-arabic-corrected-window/manifest.json)
contains raw reports, lifecycle/eligibility records, exact plans, independent
review and test output. Images and binaries remain external by hash. Replay
launches no engine, native window or capture:

```sh
python3 docs/spikes/evidence/core-arabic-corrected-window/verify.py
python3 docs/spikes/evidence/core-arabic-corrected-window/verify.py --originals --images
```
