# Windows native TypeScript compiler entry

Checkpoint 2 at `c3ec3e84dae6bf957304471639e7fac34d2297d4` failed in the
Windows Server 2025 / Node 22.23.3 workspace test job
[114525340341](https://github.com/snowdamiz/incant/actions/runs/38158170552/job/114525340341).
`native_agent_play::tests::agent_play_cooks_authored_typescript_without_rebuilding_and_failures_preserve_previous_run`
failed before the fixed compiler driver started: Node's entry resolver called
`realpathSync` and reported `EISDIR`, `lstat 'D:'`. The subsequent missing export
artifact was downstream of the failed test step, not a successful export proof.

Rust [canonicalize](https://doc.rust-lang.org/stable/std/fs/fn.canonicalize.html)
uses extended-length Windows paths. The observed entry-resolution failure matches
[Node issue 62446](https://github.com/nodejs/node/issues/62446) and its
[upstream repair](https://github.com/nodejs/node/pull/65378), which identify a
namespaced drive-root probe losing its separator. This is the supported diagnosis;
macOS cannot independently reproduce the Windows filesystem failure.

The compiler now passes its fixed `tools/build_native_script.mjs` entry relative
to the trusted engine working directory. On Windows, a small launch adapter maps
only recognized drive/UNC prefixes to regular absolute spellings, preserving
native OsString components. The candidate must canonicalize back to the exact
checked engine directory. Other namespaces, relative/noncanonical components,
or a spelling that resolves elsewhere refuse before process creation. This also
avoids relying on Windows normalizing a verbatim current directory for Node.
The canonical engine driver/dependency checks, project root and selected host Node
checks, source recovery receipts, scrubbed child environment, process deadline,
private temporary cache and diagnostic/output limits remain in place. No arbitrary
project path loses its namespace prefix; no environment inheritance was widened.
Node remains an authoring startup/compiler dependency, never a shipped runtime
or engine-agent shell capability.

The regression cooks an actual two-file TypeScript behavior from a canonical
project directory containing spaces and a non-ASCII character, then runs thirty
native ticks and checks movement of 1.5 units. It verifies unchanged project/text
and absence of generated files in the project. Windows explicitly checks that
this input uses Rust's verbatim canonical prefix. Two Windows-only unit tests
cover drive/UNC spellings, unsupported/relative namespace refusal and an actual
directory's canonical round trip. The existing failing native-agent Play test
also remains enabled, including failure preservation checks.

Local validation on aarch64 macOS, private target, frozen inputs:

- `./tools/cargo test --locked -j2 --profile ci -p incant_cook --features typescript --test typescript -p incant_headless --bin incant_headless`: 9 passed, 0 failed/ignored (2 cook + 7 headless).
- `./tools/cargo clippy --locked -j2 --profile ci -p incant_cook -p incant_headless --features incant_cook/typescript --all-targets -- -D warnings`: passed.
- Formatting, convention generation check and whitespace check: passed.
- Independent read-only review of all three source/test files: accepted, no actionable blocker.

Actual Windows execution remains required. The two Windows-only unit tests were
not executed on macOS; local results do not establish drive/UNC filesystem behavior
or make checkpoint 2 pass. Use the existing hosted workspace tests at the next
coordinated frozen candidate. No additional workflow, duplicate full CI run,
publication, release update or GPU execution is part of this patch.

## Checkpoint 3: restricted TypeScript host lookup

The corrected Node entry reached the trusted compiler on hosted Windows. The
new spaces/Unicode regression then exposed a second defect: TypeScript normalizes
callback filenames to forward slashes, while the restricted host keyed captured
sources, containing modules and the library directory with native separators.
The strict compiler therefore reported missing project files and global library
types. The failed checkpoint remains recorded and is not promoted.

The compiler host now normalizes callback paths using the same native separator
and compiler case policy as its captured keys. It still reads only captured
project/SDK text or the pinned TypeScript library directory; unknown files never
reach the filesystem. Ambiguous normalized snapshot keys fail explicitly. No
project compiler options, plugin, module loader or filesystem capability is added.

Two new drive/UNC regressions run the actual pinned TypeScript program with real
ES library text. The exact old callback block reproduces missing files/globals;
the fixed host passes, still rejects strict type errors and host globals, and
preserves denied-path and alias controls. All 19 Node tests, two actual Rust
compiler/native-motion tests, four agent Play tests and the public 32-request,
two-process source transaction proof pass locally. Runtime Rust and compiler
flags are unchanged. Independent root review accepted the exact source patch.
[Source/log identities](evidence/windows-compiler-host-paths.json) retain the red
and green evidence. Actual Windows confirmation is pending the next full frozen
checkpoint; synthetic path tests are not presented as hosted execution.
