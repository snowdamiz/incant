# Maintained QuickJS profiling on main274

The composition `d46db9c9` starts at merged main274 `b2475341`. It takes only the
accepted producer source `9c4ba9f7`, compatibility repair `9de22976`, test cleanup
`1b20ad5a` and packet `717c48ba`. All 417 incoming paths except `.gitattributes`
are byte-exact to that producer. The attributes file retains both existing
diagnostic patch exceptions and the producer's vendor byte-preservation rules.
The 44 intervening main paths are authoring migration/tests, diagnostic tools and
documentation; profile/script/player/vendor/manifests/lock have no source join.

Opt-in native profiling can now observe maintained QuickJS allocation counters
through the existing runtime guard. Detailed scanning remains explicit. Reports
identify the observation mode, invalid samples and tick-end sampled peaks;
consumers refuse cross-mode or cross-generation performance comparisons. This
does not establish lower guest execution cost, continuous heap peaks or RSS.
Existing cooked artifacts require recooking because the vendor bytes enter the
host compatibility identity. Memory limits and execution deadlines are unchanged.

Root accepted the exact source composition before current qualification. Its
prior producer final receipt is retained byte-exact as `producer-review.json` in
the current archive, SHA256
`0637a7747f030ec4f4bc0f58a4853cf62c3f2115ba82eba2e392d6bb16d9f9dc`.
The original producer report's pending-integration wording describes that earlier
checkpoint; its immutable archive and execution generations remain unchanged.
Current final packet review is accepted: retained `root-review.json`, SHA256
`0ca80b41a0175fff97b185707f25c51b15827977391e3fc4e69fa6644f6c728a`.
Scoped publication awaits the exact-head hosted fast check.

## Actual current checks

All 13 commands passed against one frozen 3,984-file source set, with Rust 1.99,
CI profile, explicit Apple arm64 target, offline/locked Cargo, jobs2 and Rayon2:

- Profile disabled: two tests. Summary: eleven tests, plus its allocation harness.
- Actual script/player profile fixtures: two tests per physics mode, including
  successful/rolled-back script ticks and child-player gameplay/PCM comparisons.
- Strict all-target profile/script/player Clippy in deterministic and fast SIMD8.
- Complete metric/consumer Python modules: 21 controls; isolated probe locks: ten.
- Vendor inverse-patch audit: 383 original files, 26 patches, 20 ABI declarations.
- Authoring-free normal dependency guard, formatting, conventions and diff check.

Thirteen executable copies were retained immediately after their commands.
All children are reaped, the supervisor is absent and the font build target is
released. There was no current native qualification failure. The initial new
packet verifier incorrectly expected the producer's historical 17 Python controls
instead of the actual current 21; that packaging failure and correction are
retained without rerunning or relabeling tests.

The unchanged producer verifier also passed independently with external files
and Git source checking: 3,325 records, 615 unique payloads, 43 commands and 54
immediate copies. Its four historical failures remain explicit. The producer's
214 script cases per mode, 21 player cases per mode, feature-off, cooked identity,
Tracy, browser and isolated core tests remain executions of their recorded
generations; this packet does not call them fresh main274 runs.

## Replay and limits

```sh
python3 docs/spikes/evidence/quickjs-profile-main274/verify.py --current --originals --local-binaries
```

The portable replay checks Git source objects, composition path sets, frozen
inputs, every current command/output/binary identity and the unchanged producer
packet. The optional switches additionally compare current files and retained
external files/copies. It does not build or execute native code.

No shipping measurement, 2ms/5% gate success, mobile execution or phase approval
is included. A later measurement must account for the changed instrumentation
identity. PR274's host-bound migration and Windows-safe test observations are
already merged; actual Windows execution of those repairs remains pending.
Checkpoint33 is independently frozen and retains its original Windows failure.
