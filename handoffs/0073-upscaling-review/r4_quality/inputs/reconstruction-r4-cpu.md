# R4 reconstruction: CPU and cooker qualification

The private producer is `72b1ba8ec351e86d66a90c401e0a75a530633616`, based on
`76be9608e99bf089fcb1e0908827330f70c28ac1`. CPU validation now passes. No R4
shader has executed on a GPU, and reconstruction, thin geometry, texture detail,
DRS transitions, C7/C9/P1 and overall appearance remain unaccepted. Earlier failed
rendered results are unchanged. This packet does not authorize publication.

This is a closure note for the immutable pre-qualification
[oracle note](reconstruction-r4-oracle.md). Its frozen text and every failed
attempt remain in the archive; the older note's pending status describes that
earlier boundary.

## Source and authorship

| Commit | Change |
| --- | --- |
| `0aed5802` | Byte-exact import of Claude's 28-path R4 packet `dc7701b2`. |
| `a6704001` | Astra's test-only f64 reference, independent scalar controls, current numerical fixture alignment and MASK source assertion. |
| `1cf45d3b` | Byte-exact Claude `f7c26dcd` identifier repair: `coherent` becomes `motion_support_coherent` in two new resolvers, plus their two trusted hash rows. |
| `c78fd69a` | Rustfmt-only six-file delta. Besides whitespace, rustfmt changes optional commas and redundant single-expression braces; the retained audit does not falsely call all six files whitespace-only. |
| `bb32bdf6` | Depth fragment now consumes the already-authored `frame.lod.x`; fix the cooker visibility contract, require buffer0/b0 and add a negative contract control. ABI label becomes `160-vf`. No shader change. |
| `72b1ba8e` | Match the renderer's cooked Depth manifest expectation to `160-vf`. |

All 21 final WGSL files equal the exact Claude import after reversing only the
two approved identifier substitutions. The final freeze retains 2,465 build
inputs and 35 changed source/doc copies. The archived freeze's scope sentence
predates the later ABI-contract repairs; the table above gives their full scope.

## Actual checks and relevance

| Check | Result | Executed source |
| --- | --- | --- |
| Default renderer library, ci | 162 passed, 43 GPU tests ignored | `c78fd69a` / source003 |
| Complete shader cooker suite, ci | 34 passed, none ignored | `bb32bdf6` / source004 |
| Renderer cooked-temporal + native-metalfx library, ci | 177 passed, 44 GPU tests ignored | `72b1ba8e` / source005 |
| Strict renderer Clippy, all targets, cooked-temporal + native-metalfx, ci | exit 0 | source005 |
| Scoped rustfmt and diff check | exit 0 each | source005 |

The two renderer suites overlap; these are not 339 distinct tests. The cooker
tests validate WGSL IR, native binding emission and SPIR-V reader roundtrips.
They do not execute Metal/DX12/Vulkan programs or establish rendered quality.

The default binary was not rebuilt at source005. Exactly four paths changed
after its successful run: three cooker files and `temporal_shader/cooked.rs`.
The default feature graph excludes the optional cooker, and that renderer module
is guarded by `cooked-temporal`. All default-selected renderer source, tests and
WGSL are unchanged. After the successful cooker run, only the renderer's cooked
manifest string changed. The verifier checks these exact Git deltas and gates;
it does not relabel earlier executions as fresh final builds.

Four failed commands remain intact: the initial default suite (160 pass, two
failures for the reserved WGSL identifier and stale MASK literal); rustfmt;
cooker stage visibility; and the cooked renderer's stale Depth manifest string
(176 pass, one failure). All four supervisor chains and all children are reaped,
with empty owned process groups and the CPU target released. Both Claude source
turns retain their original settings, tool metadata and closure records. No
extra model turn, build, timing run or GPU execution occurred during packaging.

## Numerical scope and next device boundary

The new f64 reference enumerates positive current support, clear-inclusive mass,
malformed-data exclusion, associated history before density/cap, whole-support
motion/depth coherence, owner provenance and finite age retirement. Its fixtures
have uniform native depth; it is not a general rasterizer. The independent
counterexamples and historical model-only threshold tests remain distinct.

The smallest next device scope is the ignored ordinary fixture (19 queue
submissions/maps at 8×8) and ignored owner fixture (12 at 16×16), each under
default WGSL and cooked-native Metal: four invocations, at most 62 explicit
submissions/maps, no images, stop on first failure. This is a proposal, not a
grant. Case 3 requires exact whole-plane history-enabled/current-only equality
plus independently bounded Gaussian fresh color. Owner cases 7/8 compare the
current R4 ordinary algorithm, not the old rejected algorithm. Rendered review
must follow separately through Claude Opus 5.5 with Max; CPU bounds cannot close
the historical appearance gates.

## Reproduction

From the repository root, run without Python optimization:

```sh
python3 docs/spikes/evidence/reconstruction-r4-cpu/collect.py --verify-only --originals --local-binaries
```

The portable [manifest](evidence/reconstruction-r4-cpu/manifest.json) binds
335 raw/lossless records, 152 deduplicated payloads, both executable copies,
the immutable source and all ten command receipts (six passing, four failed).
The verifier reconstructs counts from raw logs and reads Git objects, not the
current checkout. `--originals` additionally checks retained raw files;
`--local-binaries` checks the two mode-0755 copies outside the build target.
Neither option runs an executable or accesses a GPU.

External evidence is at
`~/.codex/incant-evidence/reconstruction-r4-cpu-001`. Frozen binary identities:

- Default: `103027db6cba5333af940fba57f29678b339680ea0a757ab39fff140c7155ebc`.
- Cooked/native: `7a629aca2ff959ece83b79019f88857ae45c59f72965cd870329cdbb1911f506`.

Independent root source review `72827a79` and CPU archive replay `039445ec`
accepted this scoped packet; their exact receipts are retained beside the manifest.
No main integration, performance claim or appearance acceptance is implied.
