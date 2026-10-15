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
and normal shipping readers refuse this diagnostic. The private adapter explicitly
admits budget schema 3; public export and performance admission stay unchanged.

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

The source/CPU qualification above did not execute a workload measurement. Its
collector controls include synthetic annotations; real VM controls separately
exercise actual interrupt/native-query checks and rollback. The later shipping
observation below supplies full-workload costs under a separate grant.

## One full-route shipping observation

A fresh private shipping build on producing source `4109b15c` joined main319 to
the qualified Rust source and the six-path private adapter. Its full 4,545-file
compiler/helper inventory is retained; this is not an execution of later main.
The adapter admits only explicit budget schema 3, reconstructs whole-tick
histograms and selects the same 108 whole-tick tail frames for every nested
aggregate. Nineteen pure-data controls passed, including mode/schema, source,
coverage, overflow and malformed counter refusals. Combined budget/navigation
schema 4 remains outside this adapter.

The actual Mac17,9 (Apple M5 Pro) ran macOS 27.0.1 build 26A434 with SDK 27.0,
Rust 1.99.0 and Node 24.21.0. Actual target compiler messages and codegen commands
confirm the selected default-off budget/phase features, shipping optimization 3,
one codegen unit, panic abort, debug info 2 and player fat LTO. Build-host units
are excluded from target feature admission. The private target used two compiler
and Rayon workers, completed in 301.927 seconds and was deleted before either
player launch. Seventeen managed tool children exited successfully and were
reaped. No public `export.json` or distribution eligibility is asserted.

Player SHA-256 is
`bfe73402de45a334094417b0bee4d4609b92ab4b64294fd75b8542887f07fa3d`.
All 74 original Core10k authored inputs and all 28 cooked data files matched the
historical package. Bytecode, hardening and other script metadata matched; the
conservative build identity honestly differs after the enforcement-input repair.
The old package supplies a functional oracle, not a timing baseline.

One unprofiled functional child and one maintained-heap diagnostic child each
completed all 2,156 ticks and eight authored checks. Entire logical reports
(excluding profile) and 13,798,400-byte PCM matched each other and the historical
oracle. PCM SHA-256 is
`ccabf2c68b6388ac554ac3efb5609ddf6b4a45d1a3f316891d5f344094e99ee3`.
Each child used the frozen 45-second child/90-second cooperative stage bounds,
with no retry; both exited zero and were reaped. Runtime environment follows the
existing collector and is not claimed to set a Rayon worker count.

All 2,156 diagnostic rows were complete, valid and successful, with 912-byte raw
rows, exact source/histogram correspondence and no refusal or overflow. The
observed check slots are:

| Slot | All-frame wall ns / checks | Same-tail wall ns / checks |
| --- | ---: | ---: |
| Interrupt inside callback | 2,967,958 / 11,478 | 346,738 / 602 |
| Explicit inside callback | 4,287,912 / 25,822 | 382,745 / 1,376 |
| Interrupt outside callback | 0 / 0 | 0 / 0 |
| Explicit outside callback | 2,110,812 / 10,780 | 177,465 / 540 |

Clock-attempt counts equal check counts. Overall nested check wall observations
are **9,366,682 ns / 2,156 frames = 4.34 µs/frame**, or 0.2331% of the enclosing
whole-tick wall total. On the same 108 selected tail frames they are
**906,948 ns / 108 = 8.40 µs/frame**, or 0.2290%. Callback checks are respectively
0.2383% and 0.2472% of nested callback wall totals. Tail selection retains frame
one and uses no marginal-quantile subtraction. The instrumented whole-tick p95
interval [2,940,001, 2,950,000] ns is context for this observer only, not a normal
shipping 2 ms/5% verdict.

This is a **scoped negative result**: the observed check/clock boundary is too
small to justify caching, throttling or weakening enforcement. These spans include
lock/check work, clocks and annotation overhead; they are not exclusive CPU time
or reclaimable savings. No wall/CPU subtraction, predicted optimization gain,
impossibility-of-2-ms claim or gate closure follows. The prior matrix,
ordered-query, navigation-cache and repeated-start outcomes remain separate.

The compact archive retains original CPU compile refusal and its one-line repair,
source snapshots, actual commands, compiler-record selection, raw reports and
receipts. The full original 33 MB Cargo stream, executables, symbols, cooked data
and PCM stay external and hash-bound. The verifier never launches native code:

```sh
python3 -B docs/spikes/evidence/script-budget-check-attribution/verify.py
python3 -B docs/spikes/evidence/script-budget-check-attribution/verify.py \
  --originals --binaries --git-source 4109b15c357d6377a10266f0a902d220cb9ad60f
```

Default replay rechecks embedded records and derives schema 3 attribution and
compiler/codegen acceptance with hash-bound repository parsers. `--originals`
compares retained original record bytes and rederives the exact compiler-message
selection from original Cargo output. `--binaries` additionally hashes retained
CPU/shipping executables, symbols, cooked data, PCM and the original Cargo stream.
Git/live comparison binds the 31 non-document qualification/adapter/builder
inputs; the full producing inventory remains historical evidence, not a claim
that every later source file is identical. No GPU, native window, capture or CUA
ran. Screen capture remains prohibited.

Use `--source-root` only for a checkout whose bound inputs still match the
producing source. During integration on main322, that stronger assertion
correctly refused the changed `Cargo.lock`: main321 added a localization
dependency for text tests. All other bound inputs matched. Historical Git-source,
original-record and binary replay passed without relabeling this measurement as
a main322 execution. The verifier retains its exact live-source requirement.
