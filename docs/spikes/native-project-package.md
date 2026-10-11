# Native desktop package from an authored project

`tools/export_native.py` compiles a selected authored scene and its attached
TypeScript modules into a fresh native shipping player. The trusted engine owns
the Cargo manifest, compiler and build script. Project text is bounded compiler
input; it cannot select native code, executables, build flags or bytecode caches.
The existing engine-owned Node cook now lives in the authoring-only `incant_cook`
TypeScript feature, shared by headless Native Play and the project build producer.

```sh
python3 tools/export_native.py /absolute/project.json /new/output-directory
python3 tools/export_native.py /absolute/project.json /new/window-package --mode window --camera CAMERA_ULID
/path/to/output/player/incant_game_player --package /path/to/output/player/data --ticks 20 --output /tmp/audio.pcm
```

Choose `--scene SCENE_ULID` for a multi-scene project. A window package requires a
camera and a native desktop; `--window` starts it. The exporter currently supports
host-equals-target macOS ARM64/x64, Linux x64/ARM64 and Windows MSVC x64/ARM64 build
configurations, subject to their installed native toolchains; platform execution
must be established separately. It uses existing validated model, texture and
audio caches. It does not create missing imported assets or sign a distribution.

## Boundary and publication

The build producer checks the source recovery epoch before reading the project and
after compiling, writes a cooked binary scene and bound native script program,
then publishes its build receipt last. Pending or changed source transactions
refuse export. The Python caller also checks the authoring document digest across
the actual build. No source recovery or user-file mutation is performed by export.

The fresh shipping build uses explicit target separation. Actual target compiler
commands, normal dependency closure and linked symbols are audited: authoring,
commands, CRDT, agent, source storage, JIT and source-parser dependencies must not
enter the player. Runtime bytecode reading and evaluation remain present. Crash
symbols and source maps are kept separately from the player and matched to its
actual executable identity.

Only referenced cooked payloads are copied. Bounded regular-file reads refuse
links and special entries, enforce expected sizes/checksums and inspect file
identity around the read. Duplicate asset references count/copy unique bytes once.
The four startup files are fixed names with required hashes; manifest-controlled
arbitrary paths are not accepted. The player verifies those files before output.
Models/textures/audio continue through their existing validated native loaders.

Private build storage is removed before the copied player starts. The smoke run
has no Node on PATH and uses only packaged data. The public output must be new;
publishing never replaces or recursively deletes an existing directory. An I/O
failure during final copying can leave an incomplete new directory without the
last-written `export.json` completion receipt. It is retained for inspection.

Cargo and exporter-launched tools use an owned process tree. Timeout, interruption,
failed output decoding and successful leader exit terminate descendants before
returning to temporary-storage cleanup. POSIX retains the leader PID until group
termination; Windows assigns a gated fixed Python wrapper to a kill-on-close Job
before it may launch the command. This manages trusted engine tools; it is not a
sandbox for programs deliberately escaping the group or changing credentials.

## Evidence and remaining work

On macOS ARM64 the fresh public exporter passed actual shipping-codegen, normal
dependency, no-source-parser and crash-symbol identity audits, then removed its
private build target before startup. All frozen input hashes remained unchanged.
After deletion of the disposable authoring project and all three TypeScript
files, three fresh copied-player processes each ran 20 ticks and reproduced the
expected X translations 2, 3 and 4 (tolerance 1e-9). Tampering with a copied cooked
scene failed before audio output was created. The stripped executable SHA-256 is
`7bc0765b3f894668d543611286831e4c6b0a016aefcf0168358e799fb9496f1a`.

Local validation also passed 23 cook/player tests, 47 headless tests (two GPU
cases ignored), scoped all-target Clippy and 119 Python tool tests (two
Windows-only cases skipped), including five package tests and 16 real process
lifecycle tests. Independent review resolved
unique asset accounting, publication wording and owned process cleanup. Exact
identities and observations are in [the evidence file](evidence/native-project-package.json).
 Windows/Linux execution of this new exporter, window/GPU
package execution, automated export UI, physical-device measurements, phone/store
packages, signing and a complete authoring-to-release workflow remain open. A
successful desktop package does not approve any full export or phase gate.
