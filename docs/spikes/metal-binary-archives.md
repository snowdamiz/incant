# Native Metal binary archive route

The default-off native Metal archive route passes capture, fresh-process strict
reuse, deliberate misses and invalid-cache recovery on Apple M5 Pro, macOS
27.0.1 (build 26A434). Ordinary and complete cooked-native feature modes each
reuse the actual initial renderer plus optional temporal catalog: 41 render and
two compute pipelines, with 43 `RequireHit` successes, no function additions or
fallback and unchanged archive bytes/inodes. This qualifies scoped archive
behavior on this host, without a performance or full shader-stall claim.

`incant_render/metal-binary-archives` adds a native HAL session through
`Device::as_hal`. It does not advertise `wgpu::Features::PIPELINE_CACHE` on Metal
or reinterpret wgpu's public Vulkan cache object. Existing renderer constructor
cache arguments and successful cold-completion hooks are unchanged. Linux/Android
Vulkan cache encoding and admission code are unchanged.

The renderer owns the archive; the native device retains a weak session. Standard
render and compute descriptors attach it after all descriptor fields are ready.
Successful ordinary creation adds those functions for later persistence. Native
archive creation/addition/serialization failures disable the optimization. An
attached creation error detaches the archive and retries ordinary creation once;
an ordinary pipeline error still reaches the renderer. Mesh/tile archive support
is outside this route. Native error domains/codes and attachment/add/fallback
counts are reported separately from ordinary cache read/write status.

The host admits a complete private file-URL path: held directory descriptors,
no symlinks or writable-by-other ancestors, root/user ancestor ownership, 0700
user-owned leaf, and bounded regular singly-linked 0600 user-owned files. Darwin
ACL inspection rejects every allow entry and accepts deny-only ACLs. Unsupported
ACL operations or permission failures disable persistence; they do not trigger
permission repair. Same-user hostile code and privileged OS mutation are outside
this boundary. Project/network inputs never supply native cache paths or bytes.

Darwin's `acl_get_fd_np` returns NULL/ENOENT when an existing object's extended
ACL property is absent. This was confirmed on the test host's root, ordinary
ancestors and new private fixtures, matching Apple's
[ACL wrapper](https://github.com/apple-oss-distributions/Libc/blob/71bbe350ab79eef58113991d817ccc6165061a64/posix1e/acl_file.c)
and [file-security implementation](https://github.com/apple-oss-distributions/Libc/blob/71bbe350ab79eef58113991d817ccc6165061a64/gen/filesec.c).
The corrected route requires extended-security filesystem support, obtains a
successful `fstatx_np` snapshot and queries ACL property presence explicitly.
Snapshot/property errors still refuse persistence; an absent property is distinct
from an inaccessible object. Present ACLs retain the same deny-only admission.

The initial build failed two NSURL temporary-borrow bindings; retaining the URL
owner for each full native call corrected those compile errors. The initial CPU
run passed 18 tests and failed nine: eight Metal fixtures stopped
at that ACL error, and one existing private-storage test assumed setuid survived
`chmod`. A bounded ABI control observed requested 04600 stored as 0600 on this
host. The refined test checks actual modes and records when the OS prevents that
special-bit case from being exercised. Production Vulkan admission is unchanged;
the host's normalized fixture is not evidence of retained-setuid refusal.

Identity binds engine/cook/HAL sources, lock/config files, Cargo features/target/cfg,
encoded Rust flags, compiler identity, adapter, native registry ID and OS/build.
The broad source fingerprint deliberately invalidates conservatively. Each key
has one raw archive and one length/hash/identity sidecar. Exports use a private
exclusive temporary, validate the native result, then publish each file with an
atomic rename. An interrupted or interleaved pair fails admission and goes cold.
The 64 MiB limit bounds admitted/published archive bytes, not driver RAM or native
serialization's intermediate disk use. Crashes can leave temporary files;
eviction across source/device identities is not implemented. File I/O and temp
cleanup happen only in explicit cold calls; renderer/session destruction and
frame submission do not flush or prune files.

Ordinary successful pipeline creation does **not** prove reuse. The ignored
`metal_archive_native_stage` test has externally supervised fresh-process stages:
capture both fixed descriptors, require both from the saved archive, refuse two
distinct valid pipelines even after ordinary creation warms Metal's own cache,
refuse the same pipelines with an empty archive, and refuse a disabled strict
session without ordinary fallback. `RequireHit` sets Apple's
[fail-on-archive-miss option](https://developer.apple.com/documentation/metal/mtlpipelineoption/failonbinaryarchivemiss?language=objc)
and never adds functions. Additional catalog stages exercise the real Session,
optional render/compute cold preparation, native parser rejection of a fixed
host-authored invalid cache with an intact checksum, and no publication after a
refused preparation or Drop. All ten approved fresh-process stages passed; their
exact outcomes and scope are recorded below.

`Renderer::new_requiring_metal_archive` is a host-only diagnostic constructor
available with both archive and native-compilation features on macOS. It installs
`RequireHit` before initial catalog preparation and keeps it for later optional
cold catalogs. Missing/rejected archives fail explicitly; there is no empty-cache
substitution, function capture, ordinary creation fallback or publication target.
The ordinary constructors retain Capture. A diagnostic-device error recorder and
the existing completion hook reject native misses and any observed addition or
fallback. CPU controls check those refusal boundaries; the separately supervised
catalog-reuse stage passed with the real startup and optional temporal
render/compute descriptors. It verifies strict counts, binds the actual
source/device/archive identity, and checks that archive bytes/inode
remain unchanged through construction, cold preparation and Drop. No project or
environment switch can select this diagnostic policy.

Apple's [archive API](https://developer.apple.com/documentation/metal/mtlbinaryarchive?language=objc)
uses file URLs for import/export. Xcode 27.0 headers recommend development-built
shipped archives over runtime mutation in shipping apps because storage and
corruption resilience require care. This experimental route remains default-off;
no cache-hit rate, frame-time improvement, native performance gate, or broader
Apple-device acceptance is claimed.

## Original retained qualification

The tested source is `d3b185d0ffad6ac389b7faaf7e443fa027c5755f`, incorporating
main351 (`1581ef89`) through `200f570f`. The only merge conflict retained both
Metal identity generation and the complete SSAO cooker. Native archive/HAL
policy bytes did not change during that join. Both copied test binaries bind the
actual resolved Cargo features, compiler/profile and distinct source identity.
The later evidence/documentation commit does not change that tested Rust source.

CPU006 built ordinary `metal-binary-archives,native-compilation` and the same
selection plus `cooked-native-shaders`. Both passed 30 focused cache controls;
strict renderer and explicit HAL lints passed in each mode. GPU002 then passed:

| Control | Observed result |
| --- | --- |
| Fixed render + compute capture / fresh strict reuse | Both captured; two strict successes, zero additions or fallback on reuse |
| Changed valid pipelines after ordinary warmup | Both refused with `AGXMetalG17X` code 3 |
| Empty archive after ordinary warmup | Both refused with the same native miss diagnostic |
| Disabled strict session | Two explicit `IncantMetalArchiveDisabled` refusals; zero native attachments |
| Ordinary initial + optional temporal catalog | 41 render / 2 compute captured; all 43 strict hits in a new process |
| Cooked initial + optional temporal catalog | 41 render / 2 compute captured; all 43 strict hits in a new process |
| Invalid native bytes with valid host envelope | `MTLBinaryArchiveDomain` code 1, then complete empty-archive capture and publication |

Strict catalog reads never published and preserved raw/sidecar bytes and inodes.
Stored catalog archives were 757,648 bytes ordinary and 759,984 bytes cooked, with
distinct source/cache keys. Actual device identity was registry `4294969070`,
Apple M5 Pro, Metal. All ten stage processes and their supervisor were independently
confirmed absent. The sampled peak among those owned trees was 34,340,864 bytes;
external driver/compiler services remain outside that measurement. Process times
are resource receipts only, not a benchmark. No window, frame or GPU command
submission was needed by these pipeline-creation controls.

The [portable manifest](evidence/metal-binary-archives/manifest.json) binds 297 exact
data-only records, both earlier failures and 20 external native artifact hashes.
The verifier checks source/tool/feature identities, actual terminal outcomes,
strict hit/miss counters and independent cleanup; it never extracts, imports or
executes archived code. Nine verifier controls include coherent attempts to turn
fallback, missing archives, disabled calls or a substituted source into success.
Run `python3 docs/spikes/evidence/metal-binary-archives/verify.py`; optionally add
`--external-root` with the retained root named in the manifest to hash native
artifacts without executing them. The frozen original source inventories and
native artifacts remain retained outside the repository.

Coverage is initial renderer and explicitly enabled temporal preparation only.
AO, bloom, vendor effects, mesh/tile descriptors, all per-level recorded keys,
asynchronous fallback, other Apple devices, reference phones and shader-stall
performance remain separate work. The native parser allocation/intermediate
serialization and external-service accounting limitations above remain in force.

## Main353 composition qualification

The joined source `505d05e988eacc5a7e65a5b503193e97c00745d0` incorporates main353
`f5eaae45`, retaining both archive identity generation and the complete MetalFX
boundary cooker. CPU007 rebuilt and copied the ordinary and complete cooked
binaries; each passed 30 cache controls and strict renderer lint. Both CPU006 HAL
lints were reused after comparing all 94 HAL files, lock/config/tool identities
and each 131-node resolved dependency/feature graph. All 51 owned CPU-process
identities were independently confirmed absent.

GPU003 repeated all ten controls in fresh processes on the same Apple M5 Pro and
OS. Ordinary and cooked catalog reuse each reported 41 render and two compute
strict successes, with zero additions/fallback/publication. Changed, empty and
disabled refusal and native-invalid recovery passed again. All ten native
processes and their supervisor were independently confirmed absent. The cooked
feature set includes `cooked-metalfx-boundary`; these archive tests still prepare
only the initial and optional temporal catalog, so they make no MetalFX boundary
or vendor-scaler execution claim.

The [composition manifest](evidence/metal-binary-archives-main353/manifest.json)
adds 143 exact data records and 16 external artifact hashes, binding the unchanged
original packet instead of replacing its history. Its data-only verifier checks
the source composition, reused HAL proof, actual terminal outcomes, native
strict-hit/refusal counters, cumulative GPU budget and both independent cleanup
reviews. Eleven controls include changed HAL features and an attempted budget
reset. Run `python3 docs/spikes/evidence/metal-binary-archives-main353/verify.py`;
the same optional `--external-root` hashes retained native files without running
them. No native build or frame is part of evidence verification.

The original target and aggregate account were reused without resetting earlier
failures or resource charges. GPU003's 4.002848500-second phase receipt leaves
170.512519958 seconds of the original native-control allowance; these are resource
records, not performance measurements. The default-off policy, catalog scope,
native-parser/external-service limitations and all unpassed gates remain unchanged.
