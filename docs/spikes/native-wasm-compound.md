# Complete compound animation through native Wasm

Operations 47–52 add pose-v5, IK-v2 and graph-v2 length/read pairs. Operations
0–46 and all prior wire records remain unchanged. Poses preserve independently
evaluated clip, spherical-pair and quaternion-mean groups, followed by an outer
TRS crossfade; they are never flattened. Each compound side has at most eight
samples. A general standalone weighted pose retains the existing sixteen-sample
typed contract; authored graph states remain limited to eight.

IK-v2 includes the complete pose and ordered model-space constraints. Graph-v2
includes captured outgoing clocks and current shared parameters. Controls use
the existing accepted-tick transaction. Sparse owner/descriptor membership,
all numerical values, inactive endpoints and both nested frames validate before
borrowed SDK views. Legacy readers refuse incomplete representations, including
after a complete cache is populated; caught active errors poison the whole tick.

Source review accepted all 40 frozen inputs. The mechanical renderer correction
restores accepted 21791d21 bytes after a duplicated automatic merge block; no new
renderer behavior belongs to this protocol. See the [source review](evidence/native-wasm-compound/source-review.json)
and [source inventory](evidence/native-wasm-compound/source-freeze.json).

The final combined actual Wasmtime/wasm2c suite passes 436 tests across 77 result
groups, with no failures or ignored tests, plus eight-package all-target Clippy.
It prepares 190 modules and checks 951 generated files. The focused compound
cases exercise standalone v5 and IK-v2 for 96 ticks in both hosts, eighteen
rollback guests, and 39 raw import modes including fourteen incomplete/legacy
length/read refusals. Separate renderer CPU tests pass 61 tests, with 32 GPU tests
explicitly ignored. The wasm32 build, normal-runtime dependency guard, generated
SDK checks, 27 Python Wasm tests, formatting and conventions checks pass.

The ordinary public exporter cooks a real imported eight-clip rig and authored
Rust module. Public commands, undo/redo and journal reopening are exercised before
the disposable authored rig/module/project/cache/guest and export build inputs,
private WABT tools and debug maps are deleted. Three fresh packaged runs each
complete 56 ticks. Each report contains 896 log records, 21,504 scalar comparisons
against the typed native oracle and 256 public vector checks. All reports and
358,400-byte PCM outputs match one another and the Wasmtime development run.
Twenty-three negative runs require their exact diagnostic, nonzero exit, empty
stdout and absent PCM/save output. These package refusals complement the actual
host tests that verify staged rollback. The shared engine repository and compiler
installation remain; this is deletion of the proof's own inputs, not a claim that
the development machine contains no source or tools.

The [durable manifest](evidence/native-wasm-compound.json) binds 41 final source
identities, twelve unchanged build inputs, 51 compressed/raw logs and 310 retained
artifacts. The accepted forty-source production commit remains unchanged;
the additional test advances an unknown-version sentinel from graph-v2 to
graph-v3 now that v2 is implemented. The earlier broad run correctly failed that
stale expectation; its log is preserved separately. Earlier fixture, build and
preflight failures are likewise retained without relabeling them as successes.
See the [final source inventory](evidence/native-wasm-compound/final-source-freeze.json)
and independent [public execution review](evidence/native-wasm-compound/export-review.json).
The [final independent review](evidence/native-wasm-compound/final-review.json)
accepts the exact source, broad checks and retained execution evidence. Publication
remains the sole coordinator's responsibility.

Whole-module checkpoint persistence remains explicitly unsupported. No GPU,
appearance, device-performance or Phase 1 gate acceptance is claimed.
