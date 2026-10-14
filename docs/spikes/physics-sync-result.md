# Existing physics tests assert successful synchronization

Checkpoint31 candidate `b99d5c4f` failed the actual macOS `checks` job
`115661202655` during workspace all-target Clippy. Three older test call sites
ignored the `Result` returned by fallible physics synchronization: character
setup, compound-contact reorder, and state echo. The exact failed job and log
remain in the portable evidence.

The repair unwraps these expected-success calls. A failed synchronization now
fails the test before later assertions; all existing state, collision and sleep
assertions remain. There is no production, dependency, lock or workflow change.
Root independently accepted the exact three-file patch
`9894d121f236dc4e272612c051341391eb50109d1eea7854b258cf0ff52a0ab6`.

Current local qualification passed strict all-target `incant_physics` Clippy,
all three affected test harnesses, scoped formatting, generated conventions and
diff checks. The manifest records exact counts, 3,495 source/build input hashes,
raw logs and three immediately copied binary identities. This is scoped package
qualification; it does not relabel the original failed workspace job or claim a
new hosted comprehensive success. The frozen candidate remains unchanged while
its other jobs finish; a reviewed main repair precedes any successor candidate.

Read-only replay:

```sh
python3 docs/spikes/evidence/physics-sync-result/verify.py
python3 docs/spikes/evidence/physics-sync-result/verify.py --originals --local-binaries
```
