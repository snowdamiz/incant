# GPU pass timestamp acceptance and concurrent frame envelope

Claude Opus 5.5 at explicitly verified Max completed the bounded 0068 r2 review
on source `f33495b2`, on Apple M5 Pro / Metal / macOS 27.0.1. All 92 actual
numerical and regression cases passed. Ten rendered PNGs were inspected by
Claude; all five retained timing-off/on pairs are byte-identical. Root separately
verified the artifact hashes, child exits and test counts. The earlier all-zero
attempt remains historical and rejected. [Complete retained evidence](evidence/gpu-timestamps-r2.json).

Three of four lit-scene samples exposed a reporting defect: a later-encoded
clear pass began before the first-encoded light assignment. Taking the first
encoded start and last encoded end therefore omitted measured work. The CPU
decoder now reports the earliest start through the latest end of every measured
pass pair. Overlap contributes once; individual pass labels and intervals remain
unchanged. This is an envelope of instrumented work, not CPU latency, presentation,
or a strict execution barrier.

Every pair must still be ordered. Odd payload counts and odd/out-of-range pair
indices refuse before indexing. An envelope wider than half the u64 counter range
is conservatively refused as ambiguous rollover; no smaller adapter-specific
counter modulus is inferred. The implementation changes no shader, query
descriptor, GPU command, queue submission or resource lifetime.

The final-source run passes ten timing CPU tests, all-target renderer Clippy and
format checks. The regression covers overlapping passes, a later-encoded earlier
start, compute finishing after display, malformed pairs and rollover between
individually ordered pairs. Independent source review accepted the patch.
[CPU source and command evidence](evidence/gpu-frame-envelope.json).

The 0068 GPU run predates this CPU-only decoder correction. Default shader features
were tested; cooked shader combinations, other GPU backends, reference-phone
measurements, actual native-window profiling and Tracy client capture remain
separate. The owner-drop fixture is only a no-panic/no-hang smoke test. No complete
profiler or phase gate is claimed.
