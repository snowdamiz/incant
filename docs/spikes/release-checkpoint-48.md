# Release checkpoint48

PR368 promoted selected main367 (`bd43d4c3c7cc9681ab734b373e8e6eec549dad55`)
after all 23 required contexts and all eight push workflows reported actual
success on frozen candidate `24c6be9bf963e04ab147fef3b8a5990de3018961`.

The release merge is `2711a5ee4dc4226c927b34965c2e8633c26ee924`. Its tree
`f801c3a1215a037705b685649bc11165c78cc04f` exactly equals both the tested
candidate and selected main. The merge retains the previous release and candidate
history and includes `Built-by: astra`.
[Exact checks, workflows and identities](evidence/release-checkpoint-48.json).

This baseline adds the reviewed opt-in DX12 pipeline libraries and their diagnostic
repairs, macOS audio-output reroute handling, recorded material prewarming,
packaged-player selected-frame capture and the earlier partial host-stage timing
observation. The full general job, desktop builds, credential persistence, Wasm
conformance, physics feasibility, platform probes, local services and website
workflow all succeeded. The website deployment job is intentionally outside the
required contexts and stayed skipped; the website build/workflow passed.

Main changes after PR367, including native coroutines, Wasm restart and the latest
sampled locale verification, remain outside this tested release tree. The newer
complete host-stage replay is separate local evidence, not part of this CI run.
Earlier failed experiments remain retained. No distribution was published, and
rendering quality, full-game performance and phase gates remain open.
