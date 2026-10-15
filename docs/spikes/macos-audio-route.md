# macOS default audio output changes

A retained Arabic native replay stopped after 77 host steps with
`Audio(Backend("default output device changed"))`. Its player exited and was
reaped; no winning screenshot or successful replay is claimed for that attempt.

The pinned CPAL 0.18.2 macOS backend emits typed `DeviceChanged` when a valid new
default exists and its DefaultOutput AudioUnit has automatically rerouted. The
patched Kira backend leaves that stream and renderer intact. Incant previously
treated this notification as a fatal error and discarded the session.

On macOS only, Incant now retains the existing mixer and counts the notification
in `route_change_notifications`. It does not recreate voices or reset cursors.
Device absence, invalidated streams, other errors and unavailable/lossy error
queues remain fatal. Each poll still consumes at most 64 events and checks queue
loss both before and after draining. Classification uses the typed error kind;
message text grants no recovery. Other platforms retain their previous policy.

All 42 audio tests, strict all-target device-enabled Clippy and the device-free
build pass. Controls cover later fatal errors, queue loss, saturation, bounded
polling, forged message text and byte-exact nonzero PCM continuation through
injected notifications using the real offline Kira mixer. Independent source
review found no blocking issue. Rustfmt subsequently wrapped one test assertion;
the retained tested and formatted patches make this difference explicit.

The [manifest](evidence/macos-audio-route/manifest.json) binds source, dependency
identities, commands, logs, review and the original failure in the compressed
records archive. This is CPU qualification, not a live device-switch or audible
continuity test. Historical shipping players are unchanged; fresh native output
and full rendered-game performance remain separate requirements.
