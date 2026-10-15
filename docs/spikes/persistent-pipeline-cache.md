# Persistent Vulkan pipeline cache

The opt-in `incant_render/persistent-pipeline-cache` feature supplies one driver
cache to the ordinary startup catalog and retains it for explicit optional cold
preparation. The current ordinary 1x catalog has 31 render pipelines and one
light-grid compute pipeline. Loading precedes construction; saving follows both
the successful device validation scope and the complete Rust preparation result.
Neither operation enters frame submission or renderer destruction.
Default builds retain their existing behavior.

The admitted storage path is Linux/Android Vulkan when the adapter supports
`PIPELINE_CACHE`. It uses an absolute `XDG_CACHE_HOME`, or `HOME/.cache`, followed
by `incant/pipelines-v1`. Android hosts without either approved application path
report `NoDirectory`. No project file, package or public caller can supply cache
bytes or a cache path. Metal, browser and unsupported adapters report
`Unsupported`; other native Vulkan storage reports `UnsupportedStorage` until
its ownership/ACL policy is implemented. This is not Metal binary archives or
DX12 pipeline libraries.

`Renderer::pipeline_cache_report` distinguishes missing, rejected, inaccessible
and supplied data, and the latest write attempt after successful cold preparation. `Supplied` means bytes reached
wgpu, not that the driver reported a cache hit. Initialization still succeeds
when optional storage cannot be read or written. The driver may decline to
export data, reported as `NoData`.

## Persistence and trust boundary

The filename hashes length-delimited backend, adapter, driver and wgpu identity.
The versioned envelope checks its identity, exact bounded length and payload
hash. Only the driver's `get_data()` output is written; invalid envelopes never
reach unsafe driver-cache admission. Payloads are limited to 64 MiB per entry.
wgpu additionally validates its own version/device header and permits a cold
fallback. The envelope checksum detects corruption; it is not authentication.

Owner-only storage supplies the trust boundary. Every operation checks a held
directory descriptor for effective-user ownership and mode 0700. Descriptor-
relative file access refuses symlinks, nonregular files, additional hard links,
foreign ownership and group/other access; reads are bounded and revalidated.
Publication uses an exclusive private temporary file, file sync, rename within
the held directory and directory sync. Failed publication removes its temporary
and leaves existing data intact where the rename has not committed. A replaced
pathname cannot redirect an already-open directory handle. Same-user hostile
code and privileged OS access are outside this boundary. Darwin and Windows ACL
admission is deliberately absent; Unix filesystem tests on this Mac do not
establish those policies or Linux driver execution.

## Optional cold preparation

The renderer owns the same device cache through bloom (7 render pipelines),
SSAO (19), motion/overlay depth (8), motion owner (4), fixed temporal resolve/copy
(2), and the temporal light-grid compute pipeline. These are existing catalogs;
shader sources, arithmetic, permutation selection and default feature admission
are unchanged. Unsupported/cooked combinations still refuse as before. MetalFX
vendor initialization is outside this Vulkan cache. The independently authored
cooked-bloom selector has a separate integration seam: its backend + cache +
`Result` constructors must keep the same cache argument and completion hook.

Every optional publication follows its existing validation scope and successful
Rust candidate. Failed preparation never requests a snapshot or writes the old
file. The driver may retain partial work in its in-memory cache; this is not a
transactional driver rollback. A later successful preparation may export those
opaque entries too. Temporal preparation still retains a newly prepared motion
prerequisite on later failure, as before, but defers its cache write until the
whole temporal request succeeds. Existing early returns do not force a write.
Bloom/SSAO settings replacement keeps its existing reconstruction behavior.

The retained resource adds one driver cache per renderer on supported opt-in
devices. The 64 MiB envelope bound is a serialized per-entry limit, not a bound
on driver-native cache memory, retained pipeline resources, multiple renderer
instances, or filesystem entries accumulated across driver versions. Snapshot
allocation and private-store I/O occur only in caller-invoked cold preparation;
these APIs must be scheduled outside gameplay by the host. No submit or Drop
flush, asynchronous compilation, eviction or new public cache authority is added.

The new CPU controls check failed-result publication refusal, unchanged typed
errors/prepared values, inert unsupported sessions and actual private-file
preservation across a late failed candidate followed by success. The Linux-only
ignored lifecycle below additionally prepares all optional catalogs and checks
71 live render + 2 live compute objects, refused settings preserving cache bytes,
bounded repeated catalogs and no Drop write. Those are live object counts, not
cumulative creation or cache-hit counters. These new controls are source-only
until the separately coordinated CPU qualification; actual Vulkan execution
remains pending.

## Historical verification

The [evidence manifest](evidence/persistent-pipeline-cache.json) binds source and
retained command logs. Fifteen filesystem/envelope tests cover cold reopen,
atomic replacement, corruption/truncation/extension, changed adapter/driver,
permission widening, symlinks/hardlinks/FIFOs, held-directory replacement,
concurrent publishers and failed-rename cleanup. The cache-enabled library has
78 passing CPU tests; the combined cooked-shader/cache library has 89, and the
default library has 63. These suites overlap. GPU tests remain ignored in these
CPU runs. Browser compilation and the authoring-free normal dependency graph
pass. All 963 locked package versions, sources and checksums are unchanged;
only the renderer's edge to the already-pinned optional rustix is added.

The Linux-only ignored test `persistent_cache` starts four bounded fresh child
processes with a test-owned cache directory. Every child forces Vulkan and
initializes the actual Renderer. Assertions require Missing → Supplied →
Rejected → Supplied, successful persistence each time. The original 30-render-pipeline assertion was
later advanced to 31 with the scene-resolve pipeline; the optional successor
adds the 71-render + 2-compute catalog checks described above. The corruption control changes only the envelope
checksum before the third process. Child logs expose adapter identity and public
status, never driver bytes. All processes are reaped; timeout/failure preserves
bounded diagnostic logs. The test's helper code is compiled on Mac, while no
Linux test is exposed to generic Mac GPU suites.

Actual Linux Vulkan execution is pending release CI. The local Docker client has
no running daemon. No cache-hit, startup-speed, rendered-parity, phone-budget or
phase-gate result follows from the CPU checks. Current fixed pipelines are still
prepared at startup; per-level permutation recording, asynchronous fallback
compilation, other backend caches and cache eviction remain separate work.
