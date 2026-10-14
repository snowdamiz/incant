# Existing-texture refresh during native Play

Implemented on main281 `dfac86cb17e9a1dd71d63207d779b3836e7ad6ae` through
source010 `af201c5b3848b9f064ea6fcb298a1b8b210fe289`. This slice refreshes
content for existing texture IDs with unchanged kind, usage, bindings and scene
topology. It keeps the running VM, native world, target provider and frame history.
New assets, changed usage/topology and other asset kinds remain unsupported.

The editor observes committed revisions through its read, mutation/history and
source-watch paths. Revisions without changed texture content are quiet no-ops.
A real texture change after incompatible nontexture edits produces a console
error and retains the complete previous live catalog. Project mutations still
pass through `incant_cmd`; the runtime receives no project document or command bus.

Preparation snapshots authority briefly, then decodes and validates the complete
catalog and prepares retained GPU resources outside the authoring lock. The
editor retains one replaceable pending candidate. Decoded payloads have a 256 MiB
limit; metadata, GPU allocations, old retained versions and allocator overhead
are additional. Publication rechecks committed revision, Play generation,
world identity, catalog generation, layout and GPU policy. A stale or refused
candidate preserves all previous owners.

The final authoring-bus → service-status lock contains only validation and
prepared handle swaps. Device polling, rendering, source IO and resource disposal
occur outside it. The ready lease and borrowed request keep live, candidate and
retirement owners outside the locked call, including pre/post-swap panic paths.
Displaced IK handles move into preallocated retirement storage before current
pose copying. The editor catches a publication panic, releases both guards
normally, then resumes unwind; Play becomes failed while authoring remains
usable and neither mutex is poisoned. Trusted native consumers must obey this
host contract; scripts receive no mutable host escape or arbitrary callback API.

Qualification used Rust 1.99.0 (`b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`),
SDK 27.0, the CI profile, arm64, locked/offline dependencies, jobs 2 and Rayon 2 in
the existing private target. The portable packet retains exact argv, tool/device
identities, source snapshots, raw logs and every source001–010 failure.

| Executed source | Result |
| --- | --- |
| source001 | Cancelled before tests after the publication ownership defect was identified; child−15, not a test result. |
| source002–003 | Production/fixture compile failures retained. |
| source004 | Texture 11 + IK 1, existing native 32 and Play 4 passed; editor compile failed. |
| source005–006 | Editor 13/14 passed; strict lint failures retained. |
| source007 | Texture 11 + IK 1 passed; editor 14 passed/1 failed on first mutex initialization. |
| source008 | Editor 15 and scoped six-package all-target strict lint passed. |
| source009 | GPU test compiled without running; test-helper lint failed. |
| source010 | GPU test compiled, scoped strict lint passed, one numerical Metal test passed. |

The source007 allocation failure remains in the packet. Source008 separately
observed first authoring-mutex acquisition 1, second 0 and empty `catch_unwind` 0;
the initialized publication path then observed allocation 0/disposal 0. Production
already acquires that same authoring mutex during editor/Play setup before a
native game is loaded. This evidence does not claim first mutex acquisition is
allocation-free. Source009/010 change only GPU controls; all 17 production files
are byte-identical to source008. Successful scoped lint retains dependency vendor
warnings and is not a claim that dependency output is warning-free.

The source010 offscreen test ran on Apple M5 Pro / Mac17,9, macOS 27.0.1
(26A434), actual Metal. It observed old-red 2304 → new-blue 2304 pixels,
final tick 2, frames 6 and presents 6. Assertions cover late same-tick GPU-policy
refusal and old resource retention, source/cache independence, no tick/draw/frame
advance during publication, next-redraw pixels, used-ticket refusal, old sealed
version validity, the same VM continuing at tick 2, and one target-provider drop
at stop. Its 32,931,480-byte executable SHA256 is
`e4f78824e524b0cae5154fd50e637058d8847d8857b531915fee5c873a8f8078`.

Main283 `1b8cb73cb24b550f6984f675d49b25305271f748` adds 15 paths of optional
script/runtime/profile diagnostics relative to main281. Those are real source
differences; this packet does not claim runtime-wide equality. All 23 curated
texture/editor paths match composition `eafc1e79845ddb490468bdc630344e041ac08585`.
Root accepted two focused CPU composition checks; they were not executed in
this producer receipt. No repeated GPU run is recorded for that composition.

Replay without native execution:

```sh
python3 docs/spikes/evidence/native-texture-refresh/verify.py
python3 docs/spikes/evidence/native-texture-refresh/verify.py --originals --binaries --source-root "$PWD"
```

The packet is CPU/source and numerical Mac GPU qualification only. A real native
window watching/importing/Undo/Redo end-to-end run, rendered appearance review,
other platforms/devices, reference-phone gates, shipping performance and the
remaining asset hot-reload behaviors are unverified. No timing improvement,
ScriptTick budget result or phase gate is claimed.
