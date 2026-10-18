# Explicit untrusted Rust module build

Source implementation for PLAN 6.3's module-build isolation milestone. Selected
Mac compiler/request and revised network-I/O controls passed in the scoped
episodes below. The original socket-creation criterion remains failed; this is
not a blanket security guarantee or cross-platform qualification. A separate
ordinary public import/selection/history follow-up also passed with the exact
service-generated module and a retained, source-compatible headless producer.

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
untrusted write scope. The Mac policy denies by default, requests `network*` and
`system-socket` denial, and restricts arbitrary exec; it grants read access to the pinned toolchain, vetted SDK,
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

Source qualification on 2026-10-18: the initial 37 selected pure controls passed.
All 48 selected pure controls passed after adding the network-I/O successor
protocol and mocked endpoint-ownership controls;
its pure command is `python3 -m unittest tools.tests.test_rust_guest
tools.tests.test_rust_guest_build tools.tests.test_rust_guest_network_control`.
This includes the exact historical SDK argv/cache digest control. Python syntax,
the trusted OS canary's Rust formatting, generated conventions and `git diff
--check` also passed. No compiled result is attributed to these checks.

The proposed actual route is `tools/probes/rust-guest-sandbox.py --plan PLAN
--grant ROOT_GRANT`. Its finite episode includes an actual relative-module SDK
compile and normal cook; Cargo/build-script/proc-macro and private-include
refusals; unknown-import cook refusal; compiler cancellation; and one trusted
native OS canary testing private read/write, unrelated exec and network-I/O
denial. The revised canary uses only owned IPv4 loopback endpoints, no DNS or
external address, and never listens. Its policy substitutes exactly its reviewed
executable for rustc and records that difference.
The production compiler include control also tests the real compiler policy.

Proposed initial bounds are 180 seconds aggregate, 90 seconds per child, one
active compiler, Rayon 1, 512 MiB private retained scratch, 16 MiB diagnostic
streams combined/4 MiB each, 1 MiB raw Wasm, sampled 8 GiB RSS and a 6 GiB disk
floor. No shared Cargo target, external network traffic, windows, GPU, capture,
audio device or performance measurement is required. Retained partials and every failure
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
The separately bounded cfg-only preflight succeeded under source `a6051dc0`;
this qualified startup only. The later full suite preserved an E0428 fixture
callback collision and an oversized ordinary-Wasm proc-macro fixture, then
corrected those fixtures without changing production admission or the raw cap.


Under source `717c6ecb`, actual session 43261 exited 1 after 10.893011084 seconds.
All eight request cases met their exact expectations: relative SDK compile,
normal validator and raw publication succeeded; Cargo/build-script admission,
private include, actual procedural macro, missing macro dependency, unknown
WASI import and cancellation refused without publishing failed children. The
trusted canary denied outside reads/writes and unrelated exec, with a clean
environment, but `socket(AF_INET, SOCK_STREAM, 0)` succeeded. It immediately
closed that socket without bind/listen/connect/send. The original creation-denial
criterion and failed result remain preserved. All 40 observed PID/groups were
independently absent; all 878 input bindings and source remained unchanged.

The **explicit successor criterion revision** is network-I/O denial, not socket
object denial. [Apple's published XNU socket policy hooks](https://github.com/apple-oss-distributions/xnu/blob/xnu-12377.1.9/security/mac_socket.c)
separate create/bind/connect/listen/send. [The syscall paths](https://github.com/apple-oss-distributions/xnu/blob/xnu-12377.1.9/bsd/kern/uipc_syscalls.c)
check bind/connect/listen/send before their operations. These public hooks do not
publish the current private Seatbelt mapping and are not proved identical to the
installed kernel. [Apple's App Sandbox guidance](https://developer.apple.com/documentation/xcode/configuring-the-macos-app-sandbox)
describes network client/server access in terms of outgoing connections and
listening. Installed `system.sb:291–294` filters `system-socket` to AF_SYSTEM and
AF_ROUTE, while `com.apple.smbd.sb:33–38` separately admits bind/inbound/outbound.
Those observations explain why creation alone is insufficient evidence; they do
not prove any unexecuted operation is denied by this policy.

The successor requires **each** actual TCP connect, TCP bind, UDP bind and UDP
send-to to return -1 with EPERM or EACCES. Refused connection, timeout, invalid
argument, unavailable route, success or another error cannot satisfy it. Socket
creation is reported separately. The parent holds one non-listening TCP loopback
reservation and one UDP loopback sink on fresh ephemeral ports, with descriptors
non-inheritable; the child uses only 127.0.0.1. No datagram, even empty, may reach
the sink during its bounded 50 ms read. Every owned descriptor closes on success,
allocation failure or child/receipt error. Listen, IPv6 and Unix-domain behavior
are not separately tested or claimed. Production policy remains unchanged.

The first revised canary-only follow-up needed one new trusted compile because
its source changed. The eight passing request observations stay bound to their original
failed full episode and unchanged service/SDK/request bytes; they are not
relabeled as a new complete producer. Cumulative native time is 20.269418043
seconds, leaving 159.730581957 of the original 180. All prior outputs/logs and
resource charges remain retained. At that checkpoint the revised native checks were unexecuted pending a separate
grant; the later scoped result is recorded below.


The first revised canary-only attempt compiled the new trusted control under
source `7c18d1d8`, then failed before sandbox execution: optional
`getsockopt(SOL_SOCKET, SO_ACCEPTCONN)` returned ENOPROTOOPT (42). Both owned
loopback endpoints closed, and all three observed PID/groups were absent.
The compiled 602,400-byte canary remains retained with SHA-256
`2fd5c90f31a9007259ac880eb6b41fd7da87a454bd6ad70214e639a2538ba1c6`.
No revised network-I/O assertion ran or passed in that attempt.

The controller replaces this optional inspection with direct AF_INET stream/
datagram ownership recording; the trusted source makes no listen call. The
successor may reuse only that exact compiled image, with current canary source
hash, exact successful compilation argv and terminal receipt all verified.
Changed bytes/source/recipe refuse before socket setup. No compiler, production
policy or actual I/O-denial predicate changes. Native time is now cumulatively
20.790128418 seconds, leaving 159.209871582 of the original account. The next
canary-only execution was then held for its exact review/grant; its result follows.


The separate revised network-I/O follow-up succeeded under controller source
`6e265da1`. Actual tool chunk `75cda4` exited 0; the reused canary image was built
under `7c18d1d8` and rehashed before/after. TCP connect, TCP bind, UDP bind and UDP
send-to each returned -1/EPERM (1). All four sockets were created successfully,
explicitly reported as creation permitted; outside read/write/exec and environment
controls also passed. The owned UDP sink received no datagram during its bounded
50ms read; both loopback descriptors closed. Wrapper 65332 and canary 65341 were
independently absent, with all 1,150 source/tool/retained-input bindings unchanged.
No compiler or request case was rerun in this final follow-up.

The combined observations qualify these selected controls for the tested Mac,
pinned toolchain and exact policy. They do not turn failed native005 into a
successful original episode or establish a blanket sandbox/security guarantee.
Cumulative native time at that network checkpoint was 21.793150502 seconds,
leaving 158.206849498 of the original 180. All earlier failures, raw diagnostics, terminal/group receipts,
criterion revision and independent root review are retained in the compact
[data-only packet](evidence/untrusted-rust-module-build/manifest.json). Its 604
inert UTF-8 records include source snapshots; native binaries/SDK libraries/Wasm
remain external hashed identities. The portable verifier executes none of them:

```sh
python3 docs/spikes/evidence/untrusted-rust-module-build/verify.py --self-test
```

Optional `--originals --artifacts` rehashes the retained local files. Artifact
hashing streams at most 64 KiB per read; its explicit 64 MiB per-file and 128 MiB
total ceilings accommodate the retained headless binary without embedding it.
Bounded manifest/archive reads and gzip decompression use the already verified
byte snapshot, including a path-replacement control. Thirty-one corrupt/boundary
controls cover false pass/scope claims, missing cases, errno substitution,
sink/cleanup failure, artifact substitution/size, caps, archive replacement and
the public authoring protocol below. All original 496 records remain unchanged.

## Ordinary public authoring follow-up

Actual tool session `29405` exited 0 under controller/report source `8d6dffcc`.
It reused the 63,423,424-byte headless binary produced by
`9d44372ae379c6d0372ff48854828e837f1fb747`, SHA-256
`f0fb0cf5c654dd03e4ad77e57901586cb6abb426a923dd77f86af169f272641f`.
The retained source-equality receipt covers 75 selected public-route files at
that producer, service source `6e265da1`, report source `8d6dffcc`, and integration
snapshot `08cad264`; it does not relabel the binary as a current-main rebuild.
The input was the exact 617,335-byte raw module from native005, SHA-256
`8d86dbd4e85120728eb69f9792f854242c7036443277017571552314d01a65d9`.
No compiler, SDK build, export or module/gameplay execution was repeated.

Four ordinary headless CLI processes and two RPC processes used the same default
`project.journal.jsonl`. Normal import succeeded; repeated import was a cache
hit with no revision or asset change. The normal cache capture retained the
exact service raw input and its prepared digest. Schema-validated
`command.execute` selected that imported module through `SetSceneProgram` at the
required revision, with the engine-supplied actor. Undo restored the exact
preselection project; Redo restored the selected project. An exact stale-revision
error left project, revision and history unchanged. Save and reopen preserved
the project and journal history. A malformed import, run after closing the
first RPC owner, exited nonzero with the normal Wasm-validator message and no
success JSON; the driver checked project and journal bytes unchanged and the
second RPC owner confirmed the same saved state/history. No direct document or
cache mutation route was added.

The 60-second/15-second-child episode finished in 2.618268375 seconds. The actual
outer handle was polled to terminal 0; wrapper 70299 and all six headless
PID/groups were independently absent. All 177 frozen input bindings and both
protected input artifacts were rehashed; root independently reviewed raw CLI,
19 RPC requests, saved state/history and cleanup. Sampled peak RSS was
231,735,296 bytes and allocated output peak was 4,681,728 bytes. Cumulative native
time is now 24.411418877 seconds, leaving 155.588581123 of the original 180;
every earlier failed receipt remains failed. Six source-only driver controls and
the full data-only replay with originals, external artifacts and 31 negative/
boundary controls passed. No new source-free export, browser, performance,
native window, physical device or cross-platform isolation result is claimed.
