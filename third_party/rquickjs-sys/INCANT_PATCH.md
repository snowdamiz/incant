# Incant maintained allocation observation for rquickjs-sys 0.14.0

Published crate archive SHA-256: `cee271d0eeba64f0915b846cb7ae02e16faf3dfdffdca91731101d9d30fe3423`.
Upstream rquickjs Git commit: `d7ef5eeae702fea24c03643064de454f1c1dd4b0`. The registry archive's
files are retained except registry-only `.cargo-ok` and `.cargo_vcs_info.json`.
`INCANT_UPSTREAM.json` binds every retained upstream file and each modified file;
`INCANT_PATCH.diff` is the complete source delta. The upstream MIT license is
retained as `LICENSE`, fetched from the exact commit URL recorded in the manifest.
QuickJS's separate upstream license remains at `quickjs/LICENSE` in the sys crate.

The sys patch adds `JS_GetMallocState` with three caller-owned `size_t` outputs:
logical allocated bytes, logical allocation count and the current allocation
limit. It copies maintained fields only. There is no allocation, object scan,
GC, allocator replacement, bytecode/VM behavior change or private-layout cast.
All 20 bundled ABI declarations use their existing target-specific `size_t`.
Existing mobile bindgen reads the additive header. Other-target execution is not
claimed by these declarations; compile qualification is recorded separately.

The core patch adds owned `runtime::MallocState` and sync/async runtime methods
under the existing exclusive runtime guard. It does not expose raw pointers or
add an allocation callback. One private native parity test retains the full scan
under the same guard; the earlier isolated diagnostic's 32 observations, OOM,
arena, external/owned buffer and independent-VM controls remain separate evidence
at `docs/spikes/quickjs-malloc-state.md` on producer 57897285.

Logical allocation excludes unused small-block arena backing and differs from
`memory_used_size` (an estimated object walk that can include externally owned
ArrayBuffers), Rust requested allocations and process RSS. Neither getter nor
full scan is a continuous peak tracker. The shipped default libc allocator and
ScriptHost's 32 MiB limit are unchanged. Alternative upstream allocator APIs
retain their own upstream accounting semantics; they are not the Incant host.

Maintenance: this is an additive pinned dependency patch, not an upstream release.
On upgrade, recheck native field meaning, generated ABI declarations, runtime lock
ownership, exact full-scan parity and limit/OOM behavior. Keep upstream security
updates tracked; no weakened sandbox or arbitrary-bytecode admission follows.
See `docs/spikes/quickjs-maintained-profile.md` for qualification status.
