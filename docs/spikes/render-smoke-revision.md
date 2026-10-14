# Preserve renderer smoke revisions across RPC reopen

Source `0862232f767ef67ffa9bff64f999373b3956925f` starts at main278
`0b57cc37db409cd228a7332aa42e2c9dad02ac2e`. Checkpoint35's retained Linux
desktop log failed at the second edit in tools/platforms/render-check.py. The
helper sent expected revision 0 for both edits. The host-bound application RPC
now replays the existing journal, so the second process opens revision 1.
The helper also discarded the structured RPC error before its traceback.

The first edit now returns the successful command revision and the second edit
uses it. No retry or journal reset was added. The helper still requires two
successful command/save receipts; it refuses malformed revisions, IDs and save
results. Failure diagnostics retain the native error and explicitly report
whether its code field was present. Native RPC currently supplies a message
without a code; the helper does not invent one. All PNG, imported-model counters,
source-removal/reimport and hierarchy-versus-flattened assertions remain.

The producer combined command ran nine Python controls: six methods from
tools.tests.test_render_geometry and three from tools.tests.test_ci_profile.
The profile-test file, Cargo profiles and both CI workflow inputs are frozen to
source 0862232f. The controls include the actual main/edit call chain
with synthetic PNG transport, revision chaining, explicit custom journal use,
malformed receipts, error preservation and all existing model counters. Mocked
transport does not count as a renderer or hardware gate.

The bounded actual authoring protocol also passed on the retained historical
main275 application CI-profile executable, SHA256
`b6c19fef570f817e30d6c0f73e982eff941e10840480a653d9e625b079df8f71`.
Its producing receipt and source relevance are recorded: headless/source
production and Cargo/lock/toolchain match the current branch; the command delta
contains only test isolation. The original preflight queried src directories;
the supplemental complete-crate delta also records the test-only
crates/incant_cmd/tests/prefabs.rs change, seven paths in total. The original
preflight remains unchanged. This is not the current whole-runtime executable.
Four processes initialized two entities, committed/saved revision 1, reopened
and committed/saved revision 2, then refused expected 0 while project.read kept
revision 2 and exact project bytes. The native error was `document revision
conflict: expected 0, current 2`, with no code field. No renderer, GPU, build or
performance execution occurred in this producer control.

Root separately ran the six geometry methods with a focused discover command,
then executed the complete helper once on that same historical Mac
binary and accepted the source. All unchanged PNG/geometry, hierarchy equality,
model import, source removal and reimport assertions passed. The retained launch
binds helper source 0862232f and its exact hash; both root commands exited 0 and
their owned process groups were reaped. This is scoped local functional evidence,
not current whole-runtime qualification, visual review or a performance result.

```sh
python3 -B docs/spikes/evidence/render-smoke-revision/verify.py
```

The small portable packet contains exact helper/test bytes, nine-test logs, raw
RPC inputs/outputs, producer identity/source relevance, the original Linux
failure excerpt and root's independent Mac receipts/report. `--originals --binary`
also rehashes retained external files and the historical executable. Replay
only reads data and does not build or run native code. Hosted Linux repair
execution remains pending; checkpoint35's original failure is retained. No
shipping timing, speedup, phase approval or unlisted platform result is claimed.
