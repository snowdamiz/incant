# Project-bound announcement correction

Status: implemented, focused checks passed, source frozen and released for root review. Astra authored only the four paths listed in the manifest. No commit, publish, merge or phase claim.

`Shell.announce` now returns a revocation callback for only its own announcement. A new generation cancels both earlier timers; generation checks prevent an old callback or revocation from showing/clearing a newer unrelated message. Revocation clears the queued or visible announcement, and Shell unmount cancels its timers.

`ProjectFileProvider` owns its latest announcement revocation (earlier announcements are already superseded by Shell), revokes it on project/bridge scope replacement and unmount, and ignores late results after provider unmount. Existing request IDs, review IDs, focus routing, single-flight and backend operations are retained.

Observable regressions cover queued success then project switch, visible success then project switch, old revoke against newer queued/visible status, canceled earlier expiry, revocation of current queued/visible status and unmounted timer cleanup.

First focused run failed because the existing host-answer helper’s nested async real-clock settling remained open under the new fake clock, causing subsequent empty renders. That exact failed log is retained. The test helper now permits the new cases to own their async act boundary; existing tests retain the default settling. Successor: 2 files / 34 tests passed. UI typecheck passed. `git diff --check` passed.

Pre/post snapshots, separate Astra delta patch, exact hashes and retained logs are in this directory. Root’s r6 whole UI/capture checks predate this delta; root owns final review and whole-suite/exact-source validation. No style, capture, dependency, production bridge or other source path was edited by this correction.

Built-by: astra
