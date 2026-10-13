# Pipeline boundary composition with broad, narrow and CCD owners

Accepted pipeline producer `706a89a8` and isolated vendor observer `40adf519`
are composed with reviewed broad-owner candidate `d23fbeba` on actual main222.
The only source conflict was shared library module/export/error declarations;
all existing narrow, broad, pending-tree and CCD/configuration fields are retained.
All 1,558 physics/solver/build-control inputs match the accepted pipeline producer
or reviewed broad base except that explicit declaration join. No lock, root or
snapshot manifest change is introduced. Vendor changes are exactly the separately
reviewed default-off observer and its pin records; no solver algorithm changes.

The actual composition was rebuilt after removing only owned local snapshot,
Rapier and Parry target products. Deterministic and fast8 each pass 192 tests in
24 groups, zero failures or ignored cases. Strict all-target Clippy in both modes,
wasm32 compilation, the std-only normal graph, runtime dependency guard, formatting,
conventions and diff checks pass. Original explicit-parallel controls also passed,
but that first integration runner left Rayon pool size implicit. Those receipts
remain as default-pool evidence. A focused follow-up pins `RAYON_NUM_THREADS=2`
and executes both pipeline-state and trusted-pipeline tests: 20 passing cases
in each numerical mode. Their compiler artifacts verify actual parallel and
numerical features. Totals overlap the full suites; no additive coverage claim.
All fifteen command receipts are exit zero, and every child has been reaped.

[Manifest](evidence/physics-pipeline-main222.json) and
[read-only verifier](evidence/physics-pipeline-main222/verify.py) retain 37 current
records/26 payloads, the 3,143-input freeze, source mapping and raw streams. The
accepted producer's 500 records/159 payloads, historical failures and independent
final review remain intact. Integrated final evidence review is pending.

The observer borrows/copies pipeline state without joining deferred workers or
clearing quarantine/poison. The inert section preserves exact gravity and the
three-valued historical CCD verdict; it refuses deferred, quarantined, nonpositive
or unauthenticated phases. The trusted counterexample establishes a CCD cache and
substep-counter difference, not trajectory divergence or complete pipeline-phase
authentication. There is no live constructor, hostile restore admission, world-save
enablement or device/performance/phase claim. `PhysicsUnsupported` stays closed.
