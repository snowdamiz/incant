# Inactive editor-runtime instrumentation: proposed CPU comparison

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

Both proposed artifacts select the default editor's deterministic physics and
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

CPU qualification is not yet executed. It will run only the two nonignored
fixture controls and strict fixture lint in both feature modes, retaining copied
test binaries. The complete route remains ignored during qualification. Fresh
functional admission and one fixed B0,C0,C1,B1,B2,C2 sequence require a separately
reviewed plan and quiet-host grant after all native builders are reaped. No
warmup, retries, exclusions, quantile averaging or workload changes are allowed.
Per-pair raw p95 and total differences are observations; the proposed 5% bound
is a local investigation aid, not shipping or GUI acceptance.
