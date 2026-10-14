# Maintained QuickJS allocation counters: native diagnostic

The isolated getter matches the unchanged full memory scan's logical allocation
bytes, allocation count and limit at all **32 observations**. The deliberately
incorrect C getter fails at the first count comparison, proving that the tested
binary reaches the added native function. This establishes a small accessor's
correctness in the declared native configuration. It does not establish a speedup
or adopt a new profiler mode.

Source is `3d75ae61c9a2e0ce8fee6595979c5d04b0ced6cd`, based on preserved engine
`03bc4b987f3e553249846c4cfce991793d4d2c48`. Seven diagnostic files live in
`tools/probes/quickjs-malloc-state`; no normal Cargo manifest, lockfile, runtime,
profiler or guest code changed. The earlier failed Float64 store patch is absent.
The heap assessment remains separately preserved at `11dac35af6049c9ed5d91284022217f0fc11ca30`.

## Actual execution

One sequential qualification used fresh, separate getter and wrong-count targets,
Rust 1.99.0, Apple Clang 21.0.0, explicit `aarch64-apple-darwin`, profile `ci`,
locked/offline builds, two Cargo jobs and Rayon threads. The selected features
were rquickjs `default,std`, rquickjs-core `std`, and no rquickjs-sys features.
No custom allocator, parallel runtime, bindgen or modified dependency resolution
was used. All nonlocal dependency identities match the existing workspace lock.

Fourteen commands exited zero, including both builds, getter execution, strict
harness Clippy, scoped format and diff checks. The fifteenth is the deliberate
wrong-count execution: exit 101 with `count: first/before-context`, no success
output. The bounded supervisor and all children were reaped; CPU ownership was
released. There was no failed native source generation or rerun. Initial source
preflight PATH/read mistakes remain noted in the frozen source receipt.

The safe wrapper takes the existing exclusive runtime guard. The diagnostic
holds that one guard across getter → full scan → getter, compares all three
scalars exactly and confirms that neither observation mutates them. Controls
cover contexts, 10,000 small objects and arena churn, cycles and GC, real native
ArrayBuffer growth/shrink, OOM allocation and realloc refusal preserving old
buffer length/content, independent VMs, limit changes and teardown. The detailed
used-size scan remains present throughout qualification.

The unchanged 32 MiB limit rejected both oversized allocation and resize. A
failed resize can allocate exception metadata; the control promises preservation
of the original buffer, not bitwise equality of every runtime allocation counter.

## Observed metric distinctions

These values are from the retained correctness run, not timing samples:

| Observation | Native logical allocated bytes | Full-scan estimated used bytes |
| --- | ---: | ---: |
| Before external 1 MiB buffer | 101,352 | 96,421 |
| External buffer attached | 101,656 | 1,145,294 |
| Before native-owned 1 MiB buffer | 101,456 | 96,518 |
| Native-owned buffer attached | 1,166,704 | 1,145,388 |
| Before transient 2 MiB allocation | 101,560 | 96,614 |
| During transient allocation | 2,215,336 | 2,194,014 |
| After transient release and GC | 101,560 | 96,614 |

The external control uses the existing safe immutable `Arc<[u8]>` backing API.
Rust keeps a reference; JS contents are checked, the JS view is detached and
external ownership/content are verified afterward. It covers shared external
backing, **not** mutable ECS loan or rollback correctness. The used-size estimate
includes externally owned contents; logical counters do not count those contents
as QuickJS allocations. Neither metric equals RSS or allocator arena backing.

Endpoint samples miss the transient allocation shown above. No continuous peak
was implemented or claimed. Replacing full enumeration in production would need
explicit detailed-used-size availability, sampled-peak semantics and
instrumentation identity; no field can silently acquire a different meaning.
The native hard allocation limit and GC accounting do not require either scan.

## Retained source and execution evidence

Portable packet: `docs/spikes/evidence/quickjs-malloc-state/manifest.json` and
`records.json.gz` (929 records, 444 deduplicated payloads, about 3.6 MB). It includes
the exact source freeze, independent source receipt, copied crate inventories,
MIT license/metadata, patches, compiler identities, resolved features/locks,
verbose logs, actual compiled C/header copies, raw observations and expected
failure. Larger native objects and the two immediately retained executables are
external and hash-bound; the verifier can rehash them without executing them.

External root: `~/.codex/incant-evidence/quickjs-malloc-state-001`.
Getter executable SHA-256:
`09b52e1ed2b75c8fc1fa51e6d2ca385f378ae871aebf9a7897bf2c1a5dad6681`.
Wrong-count executable SHA-256:
`dad01d49189fa6bd7a092920ccdb4e98f1abec26bc469a911eb001eda32d7ccb`.
Independent source acceptance:
`0315a7820863e3c454199d4c84e4245b245f9fafdafe2258c290b9aef6ca11ca`.

```sh
python3 docs/spikes/evidence/quickjs-malloc-state/verify.py --external --git-source
```

That command passed after packaging. It replays exact patch application, source
and binary bindings, dependency identity, every command result and the raw
metric relationships. The executable's same-lock assertions remain the evidence
for getter/full-scan scalar equality; the verifier does not rerun native code.

Only the native Apple arm64 binding was added in isolated copied dependencies.
Any production patch needs supported-target bindings, normal dependency notices,
upgrade maintenance, integration review and a fresh matched measurement. There
is no shipping, frame-time, continuous-peak, device-matrix or production acceptance
in this packet. Earlier failed optimization branches remain held separately.
