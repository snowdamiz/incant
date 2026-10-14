# Hosted opaque native session checkpoints

The optional `native-checkpoints` API now composes the existing bounded typed UI
and offline audio owners with opaque physics and explicit TypeScript persistence.
`checkpoint_with_ui` and `checkpoint_with_audio` validate the actual current
owner's world identity, generation/tick and declarations. Audio additionally
requires the exact rational sample clock. A stale, unrelated, failed or device
owner refuses; the checkpoint cannot be assembled from caller-supplied bytes.

`prepare` reconstructs the native world, then binds the captured UI/audio sections
to that world's fresh identity before any VM is initialized. The returned existing
`PendingSessionCheckpoint` exposes only immutable world/section views. The host
restores required owners and consumes it using `start_with_ui` or
`start_with_audio`; owner pairing and state equality are checked before module
initialization. Candidate failures leave the source live session unchanged.

`prepare_for` additionally checks the healthy receiving program and compatible
world, refuses pending restart requests and preserves required host obligations.
Its private pending restart bit retains only that receiver's current opt-in.
Fresh and byte preparations leave it disabled. Captured action bindings are
restored, physical input starts neutral, and the host discards external queues.
The default headless reconstruction/start methods continue refusing hosted data.

An explicitly captured offline owner is retained even when the world has no audio
declarations yet. The actual mixer clock and suspension/pending transport state
must survive a later audio declaration. Such an extra section is admitted only
by the opaque native route after real owner validation, and it requires audio
restoration before startup. The public byte codec retains its original exact
world/section presence equality. This does not enable persisted physics saves.

The game-level wrapper owns sample-rate family and suspension semantics outside
this script crate. Its composition and real mixed PCM proof are separately
reviewed. Script controls use a real offline mixer with silence and CPU widget
state/layout with an explicit deterministic measurement fixture; they do not
claim audio-device, packaged font, GPU, appearance or exported-player acceptance.

## Bounds and scope

UI/audio use their existing bounded section formats and lifetime validators.
Physics has the supplied shared payload/work/allocation budgets; ECS retains its
save-size bound, and script state/VM keep existing limits. These are component
limits, not a process-wide allocation cap. Arbitrary JS heap/globals/closures,
Wasm state, device audio rewind and file-imported physics remain unsupported.

Historical source001 retains three missing-font fixture failures and the changed
headless host-refusal error precedence. Source002 corrected both, passed all
scoped tests and was independently accepted. The independent game review then
identified dropped silent mixer chronology; source003 preserves the actual empty
owner and adds a real silent-to-audio structural transition control. All earlier
logs, source generations and immediately copied library executables remain
qualified by their actual generation; none are relabeled as source003 execution.

## Final source003 CPU results

The full script suite passed 200 cases per deterministic and fast8 mode (141
library, 58 integration, one compile-fail documentation case), with zero failures
or ignored cases. The focused 14-case result overlaps those totals. Strict
all-target Clippy passed both hosted modes plus headless, UI-only and audio-only
configurations. Fourteen feature-off checkpoint regressions passed. Optional
UI/audio/static-module/Wasmtime compilation, lean wasm32 compilation, runtime
normal-dependency guard, conservative physics manifest guard, formatting,
conventions and diff checks passed. No GPU, device or ordinary exported package
was run for this script slice. The final packet retains the source002 199-case
results separately from source003's added silent-owner control.

Root independently accepted source002, source003 and the final producer evidence.
The current-main source relevance is separately recorded in
[native-session-hosted-main248.md](native-session-hosted-main248.md). See [the retained manifest](evidence/native-session-hosted-physics/manifest.json)
and its read-only verifier `tools/probes/native-session-hosted-physics/collect.py --verify-only`.
