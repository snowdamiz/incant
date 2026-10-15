# Opt-in DX12 pipeline libraries

Mac CPU controls and Windows GNU cross-checks pass. The latest hosted Windows
MSVC run also passes 40 CPU controls, four strict lints and all 16 native stages,
with real library hits and exact ordinary/cooked pixel parity on Microsoft's
software DX12 adapter. The earlier failed runs remain recorded below. Hardware
performance and the whole shader-stall gate remain unqualified. The feature is default off; shader
equations and renderer quality policy are unchanged. This does not close the
shader-stall gate in `PLAN.md`.

`incant_render/dx12-pipeline-library` enables a separate native HAL session on
DX12 devices. It does not advertise wgpu's Vulkan `PIPELINE_CACHE` capability.
The renderer attaches before its finite catalog is prepared and persists only
after successful explicit cold preparation and a closed validation scope.
Later optional catalog preparation uses the same session. Draws and `Drop` do
not read or publish files. This is independent of material permutation recording.

The native session covers `ID3D12Device2` pipeline streams, classic graphics and
compute descriptors. One mutex serializes native load/create/store/serialize,
including concurrent requests for the same pipeline. Semantic keys encode actual
compiled bytecode, the exact serialized root-signature digest and effective
native descriptor fields. They do not use labels, pointer values, raw C layout,
padding or creation order. Stream and classic graphics use the same semantic key;
the selected native load API also validates its descriptor. Mesh/task/view-
instancing descriptors explicitly refuse the diagnostic classic route.

Capture mode reports successful native hits, missing-or-mismatched entries,
other native errors, fallback attempts and successful/failed stores separately.
`new_requiring_dx12_library` is a trusted host diagnostic with no project path or
mode input. It requires an admitted existing library and actual native hits;
unsupported drivers, misses, disabled/overflowed counters and device errors
refuse. It never falls back, stores or publishes. A later cold miss leaves the
previous renderer resources in place.

The only production storage source is a fixed leaf in the current user's known
LocalAppData folder. Every ancestor is held without DELETE sharing and reparse
points are refused. The private leaf and each regular single-link file must be
owned by the exact current user, with a protected explicit DACL granting only
that user, SYSTEM or Administrators. Existing ACLs are never repaired. Fresh
objects receive the restrictive DACL during creation. Directory ancestry is
bounded to 64 components; native payloads are bounded to 64 MiB each. A fixed
`current.library` plus exclusive `pending.library` bounds stored generations and
crash leftovers. Publication checks `FlushFileBuffers` and uses a same-directory
write-through replacement; failures remain observable. These are private host
caches, not a defense against hostile code already running as the same user or
an administrator.

The length/checksum envelope binds conservative engine/HAL source, target,
features, compiler flags/compiler identity, native adapter identifiers and the
queried driver version. Native input backing is immutable and held by an acyclic `IUnknown` owner
attached with `SetPrivateDataInterface`, so its lifetime follows the actual COM library
object rather than our interface wrappers. Failed attachment and unwind release
the unreturned library before the last local backing owner. Unsupported storage or capture faults retain ordinary pipeline creation and report that outcome; they never count as cache hits.

Cooked builds keep selecting engine-owned backend artifacts. DX12 still invokes
the existing HLSL-to-DXIL/DXBC compiler before native PSO lookup. A pipeline-library
hit is therefore not evidence that shader compiler work was skipped, nor a
performance measurement. No project-provided executable shader/cache bytes enter
this route.

The Windows-only staged fixture is authored for real COM-reference lifetime,
fresh-process seed, strict stream/classic reuse, empty/missing/corrupt input and disabled/late-miss controls.
It retains numerical RGBA before assertions and requires exact cross-stage and
ordinary/cooked equality, including opaque/MASK/BLEND models, sprites and text.
Native support must succeed; an unsupported adapter is not an accepted hit or a
silent skip. Windows filesystem controls cover canonical anchors, wrong owner,
broad/null/unprotected ACLs, hardlinks, reparse points, concurrent staging and
held-ancestor replacement. Symlink controls explicitly require Windows symlink
privilege or Developer Mode. No Windows driver or filesystem execution is claimed
by source review or a Mac cross-check.


## Original local qualification

The [portable packet](evidence/dx12-pipeline-library/manifest.json) retains six
qualification attempts. On source `650583fc`, 23 ordinary opt-in and 23 full
cooked pipeline-cache controls passed on this Mac, with both strict all-target
lints and an ordinary feature-off compile. These include the actual shared HAL
semantic encoder and bounded envelope tests. They do not exercise DX12.

Final source `8f27e211` passes Windows GNU strict HAL/ordinary-renderer library
Clippy and full-cooked renderer all-target Clippy, using pinned Rust 1.99 and
MinGW. The latter typechecks the Windows ACL, COM-backing and staged native
fixtures. It does not execute them or establish MSVC compatibility. The later
COM ownership/types/fixture changes are Windows-only; the 46 earlier Mac test
executions remain bound to their original source, rather than relabeled as a
rerun of the final composition.

The first two attempts refused Cargo's selection of non-workspace HAL tests
before compiling. The third preserved the 46 passing Mac controls, then failed
on an uninferred Windows SID pointer. Later failures exposed a redundant options
initializer, test visibility and a fixture's nonexistent `Bloom::default`; all
remain in the packet. The reviewed ownership successor also binds the input
blob to the actual COM object, including failed attachment and retained native
references. This ownership behavior is compiled, not yet executed on Windows.

A real Windows follow-up must run the filesystem/COM controls and separate seed,
strict stream/classic, empty/missing/corrupt/disabled and lifetime processes in an
owned private directory. Default and cooked outputs must match exactly, and the
strict late-catalog miss must preserve prior pixels with zero fallback or store.
Unsupported library capability is a retained refusal, never an accepted hit.
No driver reuse, native pixel parity, storage durability, shader-compilation
removal, performance or whole-renderer shader-stall result is claimed here.

Replay without native tools:

```sh
python3 docs/spikes/evidence/dx12-pipeline-library/verify.py
python3 docs/spikes/evidence/dx12-pipeline-library/verify.py --originals --binaries --git-source 8f27e211
```

External binaries and full dependency metadata stay outside Git. `--originals`
and `--binaries` require those retained local files; the default replay checks
the portable records and strict qualification/source distinctions.

## Manual Windows diagnostic successor

The separate manual workflow `dx12-pipeline-library-diagnostic.yml` uses
`tools/probes/dx12-pipeline-library.py`. Its first hosted run stopped during
compilation; the second reached native startup. The third completed the full
diagnostic on the software adapter, as recorded below.
Existing release workflows and required check contexts are unchanged.
The original source and 127-record qualification packet above remain frozen.

The successor includes the exact HAL native-descriptor key source in the Windows
workspace tests against its actual descriptor type. It adds a type-only reexport
and an already-resolved Windows dev dependency, without changing key arithmetic
or runtime creation/storage behavior. The three vendor key controls are now
selected explicitly alongside COM backing, owner/DACL, hardlink/reparse and
envelope controls. Actual linked artifact features must match the ordinary or
full-cooked set exactly before a copied executable is used.

Each mode seeds its own fresh private cache, then runs separate strict stream,
classic, disabled, lifetime, corrupt, missing and empty-library processes. Native
load misses retained by capture remain distinct from hits and successful stores;
disabling refuses before another native load. The probe requires all 14 retained
initial/refusal RGBA buffers to match byte for byte, with the existing coverage
and three-color fixture checks. It records source, native adapter/driver identity,
actual hit/miss/fallback/store counters and executable hashes. Unsupported native
capability, symlink privilege or any assertion stops the sequence as a failure.

The hosted probe uses existing Windows Job Object ownership. Cleanup is confirmed
only when its original stop method returns; a cleanup exception stays unconfirmed.
There is no automatic retry. Builds use Rust 1.99, two Cargo jobs and two Rayon
threads, with a fresh 8-GiB logical target cap, 6-GiB free-space floor, 640-MiB
output cap, 256-MiB copied-executable cap and 128-MiB cumulative logs. CPU execution
has a 3,600-second deadline; native execution has 900 seconds within 4,500 total,
with 120 seconds per native child. Kernel cleanup must finish before returning.
These bounds do not claim aggregate GPU/driver memory or performance.

Local source controls passed nine portable parser/refusal and harmless process
cleanup tests, plus metadata, syntax, conventions and formatting. The first
Windows GNU test-target compile at `da7267ff` exposed latent vendor-test borrowing
and raw-pointer refresh errors; that failed attempt remains retained. The
test-only repair at `57839a92` preserves every key inequality and the production
key implementation, and both ordinary and full-cooked renderer `--tests` strict Clippy checks
then passed. The supervisor exited zero, all seven owned process groups were
independently absent, and no artifacts were copied. Final shared-target usage
was 15,765,515,633 bytes below its unchanged 16,315,702,201-byte ceiling.

These are Windows GNU compile/lint checks on macOS: zero native Windows tests
were executed. MSVC compilation, native library hits, Windows filesystem/COM
controls and pixel parity remain for the manual hosted diagnostic. No local
GPU, window or capture was run.

## First hosted run and resource-observer repair

[Run 38761380647](https://github.com/snowdamiz/incant/actions/runs/38761380647)
on exact main362 `a638241746403dcf73ac94e8fc1ac9ae67b5d26a` failed during the
ordinary MSVC build. The disk-usage observer encountered a Cargo `deps/rmeta…`
temporary directory that disappeared between directory enumeration and entry.
It already tolerated retired files, but incorrectly made a retired descendant
directory terminal. The supervisor stopped and cleaned its owned compiler tree;
all seven owned process records confirm cleanup. No native Windows tests or
cache-hit controls executed, and no Rust compiler defect is established by this
interrupted build. The [original result](evidence/dx12-pipeline-library/hosted-first-failure.json)
retains the exact commands, outcome and archive identity.

The observer now tolerates only a named missing descendant. Missing roots,
permission errors, paths outside the owned root and errors without path identity
remain terminal. Root presence is checked before and after each scan. Ten portable
controls pass, including deterministic directory/file retirement and refusal
cases; independent review found no issue. All byte/time limits, cleanup rules and
native assertions remain unchanged. This is sampled accounting of the owned
compiler tree, not an atomic filesystem snapshot. A new hosted run is required.


## Hosted seed assertion correction

Run [38764747651](https://github.com/snowdamiz/incant/actions/runs/38764747651)
on exact `fd08435a` passes 20 ordinary and 20 cooked Windows CPU controls,
including actual descriptor, COM-backing and filesystem assertions, plus all four
strict MSVC lint configurations. The first native `ordinary-seed` process then
fails: its stream/classic/compute hit counts are `(4, 0, 0)`, while the fixture
incorrectly requires `(0, 0, 0)`. All 15 managed child processes were cleaned up.
The [original result and log identities](evidence/dx12-pipeline-library/hosted-seed-failure.json)
retain that failure; no raw image was published and later native stages did not run.

A missing cache file guarantees a new library, not a miss for every request in
one startup. The ordinary one-sample material catalog requests both Opaque and
Mask for four culling/winding combinations. Those alpha modes use the same
shader and native descriptor; alpha testing is material data. After each Opaque
PSO is stored, its matching Mask request can load it in the same session.
Microsoft's [LoadPipeline contract](https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12pipelinelibrary1-loadpipeline)
checks both the name and description. A mismatched descriptor refuses;
[StorePipeline](https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12pipelinelibrary-storepipeline)
cannot overwrite an existing name. Such collisions therefore remain visible as
errors under the existing assertions.

The diagnostic now allows those real same-session hits while still requiring
nonzero seed misses, exactly one successful store per fallback/miss, and zero
native load, store, serialization, overflow or strict-refusal errors. Strict
fresh-process reuse still requires hits with zero misses, fallback or stores.
The initial report is saved before assertions for future diagnosis. Runtime key,
creation, storage and shader code are unchanged. Eleven portable probe controls
pass, including seed-hit acceptance and mismatched/error counter refusals. A new
native run is still needed; neither this correction nor the partial Windows
result establishes complete reuse, pixel parity or the shader-stall gate.

The corrected source `98450667` passes fresh Windows GNU strict test-target
Clippy for both ordinary and complete cooked configurations on this Mac.
The [exact local result](evidence/dx12-pipeline-library/seed-correction-local-check.json)
retains both commands/logs and positive process cleanup. No Windows binary ran.
The original CORE target ceiling remains 16,315,702,201 bytes; its final size is
15,772,603,362 bytes and the cumulative conservative time charge is 421 seconds
of 3,600. No target cleanup, artifact copying or budget reset occurred.

## Complete hosted native diagnostic

[Run 38770546310](https://github.com/snowdamiz/incant/actions/runs/38770546310)
on exact main365 `6f844ad76065995ec9d137036fd936038e785b07` passes the full
ordinary and cooked sequence. Its adapter is **Microsoft Basic Render Driver**,
reported as a CPU device using DX12 with driver `10.0.26100.33438`. This executes
real Windows APIs and native pipeline libraries, but is not a physical-GPU or
shipping-performance measurement.

Both modes pass 20 CPU controls and two strict MSVC lint configurations. Their
eight separate native processes each cover seed, strict stream reuse, strict
classic reuse, disabled loading, COM lifetime, corrupt input, missing input and
empty library. Each seed records 28 misses, 28 fallback attempts and 28 stores,
plus four valid same-session stream hits. Fresh-process stream reuse records 31
stream hits and one compute hit; classic reuse records 31 classic hits and one
compute hit. Neither route falls back or stores. The late-miss/disabled controls
retain seven refusals and the previous pixels with zero fallback or stores.
Native load, store and serialization error counters remain zero.

All 14 initial/refusal RGBA buffers are identical across ordinary/cooked builds
and stages, including the existing meaningful-color and opaque-coverage checks.
Every owned call returned zero and confirmed cleanup. The diagnostic took
967.828 seconds; this includes compilation and controls, not game frame time.
The [portable evidence](evidence/dx12-pipeline-library/hosted-success.json)
binds the original archive, source, exact logs, native receipts and raw pixels.
Replay it with:

```sh
python3 docs/spikes/evidence/dx12-pipeline-library/verify_hosted.py
python3 docs/spikes/evidence/dx12-pipeline-library/verify_hosted.py --originals
```

The original two failures and local compile results remain unchanged. This
success does not establish power-loss durability, hardware-driver coverage,
window appearance, skipped HLSL compilation, full-renderer cache coverage or
the Phase 1 shader-stall requirement. The optional library remains default off.
