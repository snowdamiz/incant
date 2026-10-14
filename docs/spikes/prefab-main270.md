# Linked prefab authoring on main270

The current composition is `71890c3c36e97272aa9b8678de633d42272b5e63`, an ordinary
merge of main270 `ed3999b02d2a747dee93d278a2c1c50d43f275b7` and the independently
accepted producer `f707e1eb6333a5f103b6dd6f33c83c11ad11cb1c`. All 29 producer paths
other than STATUS are byte-exact. Its retained final review is included in the
[new qualification packet](evidence/prefab-main270/manifest.json); the original
[producer packet](evidence/prefab-authoring/manifest.json) remains unchanged.
This closes the producer document's historical pending-review wording without
relabeling its executions as runs of the current composition.

Of the producer's 3,617 retained build inputs, 3,615 are exact. The two differences
are already reviewed main changes: the bounded TypeScript compiler observer and
script phase markers guarded by `cfg(all(test, feature = "profile", feature =
"compiler"))`. Newly added helper files and compiler JavaScript also remain exact
main270. The prefab change does not edit cook, native runtime or headless
production. Manifests, dependency lock, solver selection and ordinary scene
format are unchanged. No engine-version producer work is included.

Current deterministic-mode qualification passed 87 document/command tests, the
live headless prefab RPC/save/reopen/undo/redo case, and the cooked consumer case
with 60 native steps after discarding the authoring document. Strict all-target
Clippy passed the four affected document/command/cook/headless packages. SDK
freshness, strict SDK TypeScript, formatting, conventions and diff checks also
passed: nine commands in total, with raw streams, exit receipts, source hashes
and immediate executable copies retained. This is actual current composition
execution; the producer's 87 cases and its consumer runs remain separate.

The producer archive also preserves its initial test-only compile failure,
two deletion-budget counterexamples and initial lint failure. Independent replay
verified its 277 records, 89 payloads, all 3,617 Git inputs and 37 copied binaries.
The current 3,992-input freeze binds every selected crate/vendor/tool/SDK/schema
file and root build manifests. Neither a repeated alternate-physics suite nor
new rendering/shipping performance is claimed.

The end-user path is the existing atomic authoring command bus and live headless
RPC. It supports linked fixed-topology prototypes, explicit identity maps,
whole-component overrides/removal, revert, prototype value updates, detach and
complete-instance deletion. It still refuses nesting, topology changes and
external entity references. There is no runtime prefab loader or parallel GUI
mutation path. Editor UI, file migrations, full Phase 2 and release/phase gates
remain outside this change.

Replay from a checkout retaining the source history:

```sh
python3 docs/spikes/evidence/prefab-main270/verify.py --current
```

Optional `--originals --local-binaries` additionally verifies the retained local
raw files and executable copies. The portable archive always binds their hashes;
local artifacts are not silently synthesized. Final current-main independent
review and required hosted fast check remain pending before publication.
