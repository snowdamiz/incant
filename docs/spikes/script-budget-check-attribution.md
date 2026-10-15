# CPU-budget check attribution: source proposal

The proposed default-off `diagnostic-script-budget` feature observes the existing
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
These controls are authored and unrun.
No Rust build, benchmark, player, GPU, app/window or screen capture is included in
this source milestone. The standing screen-capture pause remains in force.

The hypothesis is distinct from closed matrix, ordered-query, navigation-cache
and repeated-start experiments. Prior same-frame attribution puts callbacks near
1.913 ms and numeric validation near 0.195 ms on its 108 tail frames; those are
historical means, not current percentiles or predicted savings. Validation already
runs once after all source callbacks. A future bounded diagnostic must retain all
2,156 Core10k frames and exact gameplay/PCM and use fresh honest compiler/source
identities. Stop if the observed check cost is small. Any subsequent implementation
optimization or matched shipping comparison needs its own reviewed protocol.
