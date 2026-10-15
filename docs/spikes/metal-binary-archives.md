# Native Metal binary archive route

A native test build, 30 focused CPU tests and strict renderer/HAL Clippy passed,
including the strict real-catalog diagnostic policy. The source now incorporates
main351, retaining Metal identity generation beside the complete SSAO cooker;
that integrated source awaits separate qualification. Native archive reuse/miss
controls and backend validation have not run. This default-off route
does not change any performance gate, default feature, or per-level
prewarming/asynchronous fallback requirement.

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

The initial CPU run passed 18 tests and failed nine: eight Metal fixtures stopped
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
refused preparation or Drop. None has run yet.

`Renderer::new_requiring_metal_archive` is a host-only diagnostic constructor
available with both archive and native-compilation features on macOS. It installs
`RequireHit` before initial catalog preparation and keeps it for later optional
cold catalogs. Missing/rejected archives fail explicitly; there is no empty-cache
substitution, function capture, ordinary creation fallback or publication target.
The ordinary constructors retain Capture. A diagnostic-device error recorder and
the existing completion hook reject native misses and any observed addition or
fallback. CPU controls check those refusal boundaries; native success still needs
the separately supervised catalog-reuse stage. That stage requires real startup
and optional temporal render/compute descriptors, verifies strict counts, binds
the actual source/device/archive identity, and checks that archive bytes/inode
remain unchanged through construction, cold preparation and Drop. No project or
environment switch can select this diagnostic policy.

Apple's [archive API](https://developer.apple.com/documentation/metal/mtlbinaryarchive?language=objc)
uses file URLs for import/export. Xcode 27.0 headers recommend development-built
shipped archives over runtime mutation in shipping apps because storage and
corruption resilience require care. This experimental route remains default-off;
no cache-hit rate, frame-time improvement, native performance gate, or broader
Apple-device acceptance is claimed.
