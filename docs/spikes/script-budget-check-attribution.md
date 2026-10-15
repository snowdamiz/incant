# CPU-budget check attribution

The default-off `diagnostic-script-budget` feature observes the existing
lock/check boundary, separating QuickJS interrupt checks from explicit host/native
checks and whether each occurs inside a source callback. It changes no check
location or frequency, thread-CPU clock, owner check, error latch, deadline,
enforcement mutex or gameplay algorithm. It makes no saving or 2 ms prediction.

Each frame retains four fixed aggregate slots: `interrupt_callback`,
`explicit_callback`, `interrupt_other`, `explicit_other`. Slots contain monotonic
wall nanoseconds around the unchanged lock/check, check count, refusal count and
actual thread-CPU clock-attempt count. Initialization/reset reads are excluded.
A successful check must have one clock attempt; a refused check can have zero
when owner or already-latched failure admission stops it first. Failed clock
attempts remain counted. No guest value, per-check record, heap allocation or
string enters the trace. Clocks and annotation work add observer overhead, which
stays included; wall duration is not an exclusive thread-CPU cost.

The report is diagnostic version 4, never the ordinary version 3 gate report.
Existing phase-only schema 1 and navigation schema 2 stay unchanged; budget-only
uses schema 3, and explicit budget plus navigation uses schema 4. New fields and
region identities are present only with the feature. Strict existing schema 1/2
and normal shipping readers must refuse this new diagnostic; no adapter or
shipping performance admission is widened by this source slice.

Raw and report rows retain their compile-time 1,024-byte ceiling and 4,096-row
capacity. Raw reservation and report-copy storage remain separately bounded at
4 MiB; final JSON is additional bounded-collector output. Missing/out-of-order,
nested, unclosed, stale-generation, foreign-thread and overflowing observations
refuse complete diagnostic coverage without changing enforcement. Refused checks
stay visible. Outside a trace, initialization/inspection checks are ignored on
the capture owner; foreign-thread capture activity invalidates completeness.
Owner-thread in-trace updates use TLS; out-of-trace capture admission can take
the existing profiler-state mutex to detect foreign-thread activity.

Source controls cover exact successful/refused and clock-attempt counts, actual
VM interrupts/native query boundaries, numeric rollback and detached stale views,
thread-owner and unavailable/reversed/expired clock latches, nesting, retired
captures, counter/storage overflow and allocation-free aggregate updates. The
conservative script build identity previously omitted `budget.rs` and
`diagnostics.rs`; both are now explicit inputs. A source-list control checks
same-length changes to either enforcement file change its framed identity suffix,
without rebuilding a compiler. This intentionally changes conservative
compatibility; prior compiled artifacts remain historical, not current builds.
The CPU qualification passed on source `18ab2c53`, following initial source
`7c509b2d`. The first attempt retained a test compile refusal: an unqualified
`diagnostic` import conflicted with Rust's tool module. Its one-line successor
qualified the test import; production and assertions stayed identical.

| Executed control | Outcome |
| --- | --- |
| Normal performance-reader refusals / conservative identity input sensitivity | 22 / 2 Python tests passed |
| Budget-only schema 3 / combined navigation schema 4 collectors | 13 / 18 tests passed |
| Ordinary version 3 summary | 6 tests passed |
| Actual script host, diagnostic enabled / ordinary feature off | 156 / 151 passed; one existing ignored test in each |
| Dedicated allocation executables | Three passed; both diagnostic variants observed 64,000 check/clock annotations across 1,000 complete frames without allocation or reallocation |
| Strict all-target Clippy, profile/script/player | Diagnostic plus navigation, and ordinary configurations passed |

These are repeated feature configurations, not a count of unique tests. The
ignored script test is the existing opt-in 128-tick CPU stress diagnostic.
Eight successful test executables were copied and hash-bound before target
release. Rust 1.99.0 used the arm64 CI profile, locked/offline resolutions and two
compiler/Rayon workers. Cumulative target growth was 74,090,239 bytes, below the
2 GiB bound; all managed children were reaped. The original failure remains in
the [portable evidence](evidence/script-budget-check-attribution/manifest.json).
Its [offline verifier](evidence/script-budget-check-attribution/verify.py) checks
raw outcomes, source identities and optional retained originals/binaries.

No shipping diagnostic, benchmark, GPU, app/window or screen capture ran in this
qualification. Collector boundary tests include synthetic annotations; the real
VM controls separately exercise actual interrupt/native-query checks and failure
rollback. Neither supplies full-workload costs or a performance gate. The
standing screen-capture pause remains in force.

The hypothesis is distinct from closed matrix, ordered-query, navigation-cache
and repeated-start experiments. Prior same-frame attribution puts callbacks near
1.913 ms and numeric validation near 0.195 ms on its 108 tail frames; those are
historical means, not current percentiles or predicted savings. Validation already
runs once after all source callbacks. A future bounded diagnostic must retain all
2,156 Core10k frames and exact gameplay/PCM and use fresh honest compiler/source
identities. Stop if the observed check cost is small. Any subsequent implementation
optimization or matched shipping comparison needs its own reviewed protocol.
