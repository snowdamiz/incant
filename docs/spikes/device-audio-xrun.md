# Device audio overrun notifications

A native Mac Core replay stopped after 1,444 of 2,156 requested steps with
`Audio(Backend("A buffer underrun or overrun occurred."))`. The actual
presentation sidecar finalized, but no complete gameplay report was returned.
That failed run remains evidence, not a successful replay.

The pinned CPAL 0.18.2 CoreAudio overload listener emits typed `ErrorKind::Xrun`
without stopping its stream. Kira preserves that stream. Incant previously
treated every queued notification as fatal and dropped the whole game session.
It now counts typed xruns and leaves the existing mixer and cursor intact.
Device loss, other error kinds, unavailable queues and discarded events remain
fatal in this producing revision. The later [macOS route notification correction](macos-audio-route.md)
also retains a mixer after the backend's successful default-output reroute.
The policy never guesses from message text.

Each poll drains at most 64 notifications, checks queue loss before and after,
and exposes cumulative xrun and poll-limit counters with visible saturation.
Counters belong to the current audio owner and reset when that owner is replaced;
they are not saved game state. The native window report includes these diagnostics
and explicitly marks glitch-free output unverified. A zero count does not establish
device liveness. Fatal session disposal can still discard prior counters, and the
editor does not yet display them.

Source `c926d6238416e0d7b9259e318972d1ac2915fc66` passed 39 audio and 35
composed-runtime CPU tests, strict all-target native-device/window Clippy, and
the ordinary device-free build. Nine new controls cover error classification,
queue loss, bounded polling, saturation, mixer retention and byte-exact nonzero
PCM continuation across an injected xrun notification. The independent source
review accepted all nine changed paths. Tests use the real offline Kira mixer;
they do not establish hardware recovery or uninterrupted audible output.

[Manifest](evidence/device-audio-xrun/manifest.json) binds the tested source,
copied test binaries, exact commands, complete logs, independent review and prior
failed-run receipt in the compressed records archive. A fresh complete native
replay remains required. No rendering, frame-time, audio-quality or phase gate
is passed by this change.
