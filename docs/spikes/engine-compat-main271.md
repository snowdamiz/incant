# Engine compatibility with linked prefab authoring

This composition joins independently accepted engine compatibility producer
`730a7e4523b81de29fff2fe914b567a556dfc859` to main271
`e3fd0bebd92787abad6f82f370b3d4a807b787e1`. The tested current source is
`0a2a066583bd1a1a4f548f94b45f22df072ecbac`. The original
[producer packet](evidence/project-engine-compatibility/manifest.json) and review
remain unchanged; [current evidence](evidence/engine-compat-main271/manifest.json)
records the distinct composition and executions.

The command and error enums retain both prefab operations and explicit engine
pinning. Project validation retains both families. New projects receive an exact
engine declaration; legacy documents preserve its absence. The one legacy-reader
prefab fixture explicitly clears the engine declaration in old input, and removes
it only from the serialized linked value used to isolate prefab rejection. The
actual linked-project and reopen paths remain pinned. A new joined control proves
that pin review, application, undo and redo preserve linked prefab overrides, and
that reverting a component still works after pinning. No production behavior was
changed beyond the accepted additive engine compatibility feature.

Of 3,615 producer inputs, 3,602 remain byte-exact. The 13 differences are the
reviewed current prefab implementation/test joins, existing bounded compiler phase
observer and combined generated contracts. Sixteen of 24 producer paths other than
STATUS are byte-exact; the remaining eight contain command/Project/schema unions.
The producer's single document semver dependency edge and lock entry are exact;
no registry version changed. The final 4,004-input freeze covers selected crates,
vendor code, tools, SDK, schemas, root build manifests and Cargo configuration.

A copied current headless executable generated all 74 schemas. Three provisional
schema files changed only their `oneOf` command order to match the combined Rust
enum; the SDK was regenerated from those actual outputs. This headless build used
`1c54a3c4`; its Rust/build inputs are exact to the final source, with only those
three generated files and the SDK changing afterward. Both generations and the
immediate executable are retained. A direct formatter invocation initially named
an absent local shim, before any compiler ran; the normal `tools/cargo` route
then passed. The original producer's two compile failures remain historical.

Current default deterministic native CI-profile qualification on this Mac passed:

- 96 document/command cases, including both complete feature families and the
  joined pin/override/history control;
- two headless CLI/RPC cases, exercising engine review and prefab transactions;
- two cook/native cases, including compatible-pin versus legacy cooked-byte
  equality, incompatible-pin refusal and source-free prefab continuation;
- strict all-target Clippy for document, command, cook and headless packages,
  SDK freshness/strict TypeScript, formatting, conventions and diff checks.

All nine qualification commands, the separate headless build and schema invocation
exited zero. The packet retains raw streams, source hashes and 36 immediate binary
copies. The producer's 80 document/command cases and two consumer cases remain
separate historical executions with 33 copies; no new alternate-physics suite,
shipping measurement, GPU or editor UI check is claimed.

The end-user path uses ordinary command transactions and host-selected headless
CLI/RPC inspection. Pin review is read-only and binds the exact opened source,
revision and candidate digest; applying its command changes the in-memory
project/journal, with explicit save remaining separate. It cannot authorize a
file migration or arbitrary path write. Project-file migration, create-only
backup, recovery application, older-format conversion, editor UI and phase/release
gates remain open. Release checkpoint32 remains failed, and the passing isolated
Windows cook observation is not a timeout repair.

Replay from a checkout retaining the source history:

```sh
python3 docs/spikes/evidence/engine-compat-main271/verify.py --current
```

Optional `--originals --local-binaries` also verifies retained external raw files
and copied executables. The portable replay checks both original and current Git
source identities without relabeling producer binaries. Final independent review
and the exact-head hosted fast check remain pending before publication.
