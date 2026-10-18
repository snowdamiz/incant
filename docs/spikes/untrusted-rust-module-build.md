# Explicit untrusted Rust module build

Source implementation for PLAN 6.3's module-build isolation milestone. Native
sandbox qualification is pending. No compiler, sandbox or escape-control process
has run for this change; pure controls do not establish actual isolation.

The existing `tools/rust_guest.py::compile` remains a trusted repository-guest
helper. Its SDK cache keys, fixed library recipes and argv order are preserved.
The explicit `--untrusted` CLI delegates to the shared typed
`rust_guest_build.build(Request, Tools)` service. Project input contains source
and an entry name; the engine/operator supplies tools and private storage. It
does not expose a shell, arbitrary Cargo, dependency selection, linker flags,
environment fields or project-selected native executables to an engine agent.

The worker captures at most 32 regular single-link UTF-8 `.rs` files, at most
64 KiB each and 256 KiB combined, with depth at most eight. It traverses held
directory handles without following links, checks file identities before/after
bounded reads and directory membership identities, and hashes the immutable
snapshot. Manifests, `build.rs`, special/native files and links refuse. Relative
Rust modules within the captured package work. Lexical source scanning is not
the isolation boundary.

Pinned Rust 1.99.0 compiles a `wasm32-unknown-unknown` cdylib with only the vetted
ABI/SDK closure. The SDK itself builds from engine-owned sources outside the
untrusted write scope. The Mac policy denies by default, network/socket creation
and arbitrary exec; it grants read access to the pinned toolchain, vetted SDK,
snapshot and public OS loader libraries. Writes are confined to a private run
tree. It admits only the exact rustc/rust-lld/cook executables. The cooker's file
read/mmap exception is an exact file, not its enclosing directory. HOME/TMPDIR
are private; credentials, compiler wrappers/flags and loader overrides are absent.

Apple's installed `sandbox-exec(1)` manual explicitly calls the utility
deprecated. The service fails closed on unsupported OS, absent tools or failed
sandbox initialization. It does not use private sandbox initialization SPI,
import broad system profiles or fall back to unrestricted project compilation.
Executable mappings are separate explicit rules: exact admitted executables,
pinned toolchain libraries and public OS library/framework code only. Source,
SDK data and writable run trees receive no executable mapping allowance.
Actual controls must qualify the selected OS/toolchain; this is not a portable
security guarantee or a production distribution/signing claim.

The existing `incant_wasm_cook` executable performs normal import/ABI/layout
validation under the same bounded policy. Only after compiler, validator,
cancellation, output rehash and owned-group cleanup checks succeed does the
worker atomically create a new raw `.wasm`. The metered prepared bytes are bound
in the receipt but never published as raw input. A new work directory retains
terminal failed receipts, partials and untrusted logs. Publication rolls back
only its own inode; concurrent replacement or directory replacement refuses.
Operation and cleanup failures remain visible together. A failed build never
becomes a trusted native module or a cache admission.

The public importer/`SetSceneProgram`/Undo/Redo path remains the sole project
mutation route. This slice adds no GUI/agent-only attachment path and does not
claim automatic editor live-code integration. Ordinary cooked export and its
source-free/native-descriptor checks remain unchanged.

`tools/tests/test_rust_guest_build.py` exercises bounded source capture/races,
path and link refusals, policy quoting/allowance, clean environment, worker
compiler/validator refusals, cancellation, collision, publication replacement,
resource/cleanup error preservation and failure receipts without OS children.
The two existing trusted SDK cache controls remain. None is native evidence.

Source qualification on 2026-10-18: all 37 selected pure controls passed with
`python3 -m unittest tools.tests.test_rust_guest tools.tests.test_rust_guest_build`.
This includes the exact historical SDK argv/cache digest control. Python syntax,
the trusted OS canary's Rust formatting, generated conventions and `git diff
--check` also passed. No compiled result is attributed to these checks.

The proposed actual route is `tools/probes/rust-guest-sandbox.py --plan PLAN
--grant ROOT_GRANT`. Its finite episode includes an actual relative-module SDK
compile and normal cook; Cargo/build-script/proc-macro and private-include
refusals; unknown-import cook refusal; compiler cancellation; and one trusted
native OS canary testing private read/write, unrelated exec and socket creation
denial. The canary only creates a socket; it never listens/connects. Its policy
substitutes exactly its reviewed executable for rustc and records that difference.
The production compiler include control also tests the real compiler policy.

Proposed initial bounds are 180 seconds aggregate, 90 seconds per child, one
active compiler, Rayon 1, 512 MiB private retained scratch, 16 MiB diagnostic
streams combined/4 MiB each, 1 MiB raw Wasm, sampled 8 GiB RSS and a 6 GiB disk
floor. No shared Cargo target, network access, windows, GPU, capture, audio device
or performance measurement is required. Retained partials and every failure
count against those bounds. Native qualification needs separate review/grant.

The first actual episode used source `dcc9fc5c` and ended at sandbox startup:
tool session 72517 exited 1 after 1.686923042 seconds. Compiler identification
and both vetted SDK builds exited 0; sandboxed `rustc --print cfg` aborted with
SIGABRT before project source compilation. The wrapper and all four native
PID/groups were independently absent. No module, normal-validator result or
escape-control acceptance followed. All 663 bound inputs remained unchanged.
The failed episode and original 180-second account remain retained externally.

The saved crash reports namespace 35/code 2 and an ignition/CacheFinder stack.
[Apple's XNU definitions](https://github.com/apple-oss-distributions/xnu/blob/xnu-12377.1.9/bsd/sys/reason.h)
identify namespace 35 as LIBIGNITION. [Published dyld source](https://github.com/apple-oss-distributions/dyld/blob/main/dyld/DyldProcessConfig.cpp)
shows CacheFinder invoking ignition; that published file is not established as
byte-identical to the installed dyld. Neither source nor the crash identifies
the exact denied path/operation for this run.

The installed `/System/Library/Sandbox/Profiles/dyld-support.sb` explicitly
documents libignition opening `/` as its `openat` root. The failed policy grants
only root metadata. The next candidate adds **file-read-data on literal `/`
only**, permitting the directory open/listing while descendant files continue
to require their own rules. It does not import the system profile or admit
additional Cryptex trees, descendant reads, writes or executable mappings. This
is a source-backed startup hypothesis, not a proven cause or isolation result.
A separately bounded cfg-only preflight is proposed before another full suite;
it remains unexecuted pending review, within the 178.313076958 seconds left.
