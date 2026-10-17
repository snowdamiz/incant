# Timer startup fixture lookup in shared conformance

Checkpoint53's macOS Wasm conformance job failed in the timer reload test before
its development-host startup-refusal assertion. The file loader looked inside
`INCANT_RUST_SDK_GUESTS` for `timers-start.wasm`. The shared producer compiles raw
WAT fixtures in that directory's parent; only compiled Rust guests live inside
`sdk-guests`. Static linkage found the module, and the other three timer parity
tests passed. The earlier isolated local producer placed the raw fixture inside
the guest directory, masking this layout mismatch.

The test helper now selects the Rust-guest directory for `rust-*` names and its
parent for raw fixtures, matching the shared producer and existing conformance
tests. Runtime code, startup authority checks and reload acceptance are unchanged.
The original hosted failure remains recorded; it cannot qualify release.

Local source `11a1189afc2814152f2daf9a7bdfb408642287af` passes a fresh ordinary
cooker, ten actual Rust SDK guests, the actual raw startup module and all five
timer import/parity controls through Wasmtime and wasm2c, followed by strict
lint. The raw module was deliberately placed beside the guest directory, and
its absence inside that directory was asserted before tests. This exercises
startup refusal, retained pending schedules and successful reload through the
same lookup that failed in CI. No fallback or test skip was introduced.

The bounded owner completed in 60.292 seconds; all 50 observed wrapper/command
identities and groups were absent afterward. No GPU, window or capture was used.
[Source, failure and local qualification identities](evidence/ci-timer-fixture-layout.json)
retain the distinction between local success and the pending hosted rerun.
