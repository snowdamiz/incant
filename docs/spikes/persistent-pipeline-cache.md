# Persistent Vulkan pipeline cache

The opt-in `incant_render/persistent-pipeline-cache` feature supplies one driver
cache to the complete startup catalog: 30 render pipelines and one light-grid
compute pipeline. Loading precedes pipeline construction; saving follows the
successful device validation scope. Neither operation enters frame submission.
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
and supplied data, and whether data was saved. `Supplied` means bytes reached
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

## Verification

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
initializes the actual full Renderer. Assertions require Missing → Supplied →
Rejected → Supplied, successful persistence each time, and exactly 30 render
plus one compute pipeline. The corruption control changes only the envelope
checksum before the third process. Child logs expose adapter identity and public
status, never driver bytes. All processes are reaped; timeout/failure preserves
bounded diagnostic logs. The test's helper code is compiled on Mac, while no
Linux test is exposed to generic Mac GPU suites.

Actual Linux Vulkan execution is pending release CI. The local Docker client has
no running daemon. No cache-hit, startup-speed, rendered-parity, phone-budget or
phase-gate result follows from the CPU checks. Current fixed pipelines are still
prepared at startup; per-level permutation recording, asynchronous fallback
compilation, other backend caches and cache eviction remain separate work.
