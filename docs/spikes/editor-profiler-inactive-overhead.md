# Inactive editor-runtime instrumentation: bounded CPU comparison

The ordinary editor profiler is functionally qualified separately. This fixture
investigates the cost of compiling its CPU hooks while no process or session
Capture is active. It does not add an availability or phase gate.

The ignored incant_agent_native test inactive_core_complete_route follows the
ordinary native Session startup/tick path with the retained Core assets, scripts,
2,156 recorded actions and observation assertions. One in-memory incant_cmd
SetPhysicsMode transaction derives deterministic physics from the original fast
project. It retains canonical before/after text, command/provenance and hashes,
proves exact Undo/Redo, and checks that no other authored field changes. Original
authoring files remain read-only. No physical input, device, GPU, window, capture,
listener, network model or shipping exporter is invoked.

Both measured artifacts select the default editor's deterministic physics and
the same arm64 Rust 1.99 CI profile. Only agent-profiler differs. This is a CPU
native-host proxy: it does not measure the GUI or compare numerical identity
against the historical fast4 kernel. Both arms must satisfy the unchanged Core
assertions and have identical fresh logical/PCM results. No original assertion
will be relaxed to accommodate the derivative.

A checked Instant pair surrounds only each Session::tick_recorded call. All
2,156 duration slots and sample/hash scratch are preallocated. Input lookup,
startup cooking, observations, PCM hashing and report serialization are outside
that span. Inclusive wall time still contains scheduler delays and the common
timer cost. Partial, invalid-clock, overflow and failed-world results retain raw
rows and cannot receive complete p95/total values. Startup failure may leave an
empty create-only output; the external supervisor must retain that failure.

The report includes actual PCM SHA-256, selected state, script fingerprint,
input hashes and the complete canonical derivative proof. It is bounded at
32 MiB; the canonical proof is bounded at 24 MiB before simulation. This bound
covers serialized bytes, not total live startup copies. The actual original Core
document is about 9.58 MB. The six-child supervisor must budget up to 192 MiB of
report output plus bounded logs, rather than assume small timing-only reports.

The producing source is `b329dec1d86b0c308172621f6c59fafce470223e`.
CPU qualification passed two nonignored fixture controls in each feature mode,
strict fixture lint in both modes, formatting and convention/diff checks. The
copied test executables are bound in the packet; the complete route remained
ignored until the separately granted functional and timing runs. Actual Cargo
artifact records have 546 package/kind/target keys and 568 distinct feature/profile
rows per arm. Seven rows across five packages add only the expected profiler
features. These are CI `opt-level=1` arm64 Rust 1.99 artifacts, not shipping builds.

Both minimum functional children completed all 2,156 ticks with all eight original
assertions and 17 selected entities. Their fresh deterministic logical results,
script fingerprint and actual mixed PCM hash match exactly. The 1,724,800 stereo
frames hash to `ccabf2c68b6388ac554ac3efb5609ddf6b4a45d1a3f316891d5f344094e99ee3`.
Historical fast4 numeric equality was not required. The original 74 authored files
were rehashed before and after execution; only the reversible in-memory derivative
changed physics mode.

One reviewed sequence then ran in fixed order **B0, C0, C1, B1, B2, C2**, where B
has hooks compiled out and C has hooks compiled in but no active Capture. Each
run completed the same route and matched the functional logical/PCM identity.
All 12,936 timing samples are retained. There were no warmups, retries, omissions,
outlier exclusions or averaged quantiles. The raw nearest-rank p95 and sum of the
same per-tick durations are:

| Fixed pair, OFF → ON | p95 milliseconds | Change | Total seconds | Change |
| --- | --- | --- | --- | --- |
| B0 → C0 | 4.378958 → 4.498083 | +2.7204% | 6.154046827 → 6.380884577 | +3.6860% |
| B1 → C1 | 4.441875 → 4.392459 | −1.1125% | 6.231247937 → 6.244145917 | +0.2070% |
| B2 → C2 | 4.458167 → 4.393334 | −1.4543% | 6.388109868 → 6.204436977 | −2.8752% |

Every pair stays within the prespecified local 5% bound for both quantities. This
supports that bounded comparison only. Variation changes direction between pairs;
it does not establish zero overhead, a speedup, ordinary GUI performance or the
shipping 2 ms gate. Functional-run wall times are not performance results.

The timing supervisor completed in 62.900 seconds, with at least 75,982,577,664
sampled free bytes. All six fixture children and six metadata children returned
after managed cleanup. No device, GPU, window, browser, screen capture or listener
was launched. Root coordinated the Incant lanes to stop heavy work, but the raw
before/after process snapshots retain unrelated Node/esbuild/workerd processes,
other application workers and CUA Node runtimes. The before snapshot also contains
the concluding Claude packaging process, absent afterward. The snapshots do not
sample utilization: they cannot prove machine-wide inactivity or quantify any
interference. None of these observations caused an exclusion or rerun.

## Retained failures and evidence

CPU plan001 was superseded before execution by a refreshed target binding. The
first CPU execution (002) failed locked dependency admission because the added
test-only `sha2` lock entry was ambiguous. Source successor b329 disambiguated it
to the already resolved version; CPU003 then passed without changing fixture
assertions. The first functional preparation also refused an incorrect assumption
that each Cargo target had only one feature/profile row; distinct rows are now
preserved rather than unioned.

The first OFF child succeeded, but its original analyzer rejected the canonical
derivative because serde omits the default deterministic physics value. That
analyzer failure and complete OFF report remain intact. The corrected analyzer
normalizes only that documented default, retains every other exact field check,
and has positive/negative controls. OFF was reused, not retried; only the previously
unexecuted ON child ran afterward. Both reports and all six later timing reports
are included byte-for-byte in the compact archive.

The [portable packet](evidence/editor-profiler-inactive/manifest.json) includes
both 88-file source freezes, CPU logs/artifact identities, all protocol generations,
failed and successful receipts, raw reports, original assertion inputs, process
snapshots and the independent raw timing replay. Reports use individually bounded
gzip payloads so the outer archive stays small without discarding canonical
documents or timing samples. No archived script or executable is run by replay:

```sh
python3 -B docs/spikes/evidence/editor-profiler-inactive/verify.py
python3 -B docs/spikes/evidence/editor-profiler-inactive/verify.py \
  --originals --binaries --external-inputs --git-source
```

Default verification checks portable raw durations, nearest-rank calculations,
same-run sums, fixed pairs, recorded PCM/logical identities, original gameplay
checks, source copies and actual feature-unit records. It verifies PCM hashes
recorded while the fixture mixed samples; raw PCM samples are not embedded.
Optional checks require the retained external originals, copied binaries, authored
assets/compiler dependencies or historical Git source. The outcome prose is a
successor to the frozen proposal; historical source verification uses the exact
producing commits and does not relabel the updated document as executed source.

## Manual fixture selection after measurement

A source successor gates the entire `inactive_core` integration-test target behind
the default-off `diagnostic-inactive-profiler` feature. Broad default workspace
`--ignored` CI therefore cannot select the external-input route. The gate does
not enable `agent-profiler`: OFF and ON remain separately selectable, and no
library/runtime implementation changes. The two self-contained fixture controls
also live in that explicit target.

To run only those controls, select `--features diagnostic-inactive-profiler` on
`cargo test -p incant_agent_native --test inactive_core`; add `agent-profiler` for
the ON variant. The ignored full Core route still requires a frozen input plan
and separately coordinated execution. Listing it does not execute it:

```sh
./tools/cargo test -p incant_agent_native --test inactive_core \
  --features diagnostic-inactive-profiler --locked --offline -- --ignored --list
```

The original b329 artifacts predate this selection-only gate and remain the
producing identity for every measurement above. A new selector qualification
must not be described as a repeated Core run or timing experiment.
