# Numeric validation basis-cache candidate

Source `66c5d14761caf61f5d2e3be90313bc5dc27bf715` adds one fixed-size, per-call
entry to numeric validation on baseline `03bc4b987f3e553249846c4cfce991793d4d2c48`.
It remembers the exact seven rotation/scale input bit patterns and the three
basis columns calculated by the existing `scene::matrix` function. On a match,
the fourth column is formed from the current translation and `1.0`, exactly as
pinned glam 0.32.1 does. A miss invokes the unchanged matrix function.

The cache is local scratch, not an entity/epoch cache or saved world state. It
allocates nothing and disappears on return, error or unwind. Every component,
UI and physics validation still executes before matrix preparation. The existing
parent-first traversal, first-error order, root identity multiplication, parent
multiplication, finiteness check and derived-state publication remain unchanged.
The ordinary simulation preparation and unchecked cleanup paths still use the
original matrix function. Signed-zero input bits are distinguished explicitly.

This candidate follows the accepted diagnostic-only script-phase experiment:
numeric validation averaged 0.253436 ms in its CI, single-source fixture. That
number does not establish a shipping cost or a benefit for this candidate.
The failed cursor/inspector candidates are excluded. Workload, deadlines, budgets,
profiler and all shipping gate criteria remain unchanged.

## Correctness qualification

Root independently reviewed all four owned files, seven baseline files and three
pinned glam files at source002. Receipt SHA256:
`ccb8384f69ab521091c3a7b1ac13751ce5194ee0127abeb7608a340fb0ab8719`.
Source001 is preserved; its only superseding change was making one new test
closure binding mutable, caught before compilation. There was no failed native
execution or production correction between those freezes.

Both deterministic and fast SIMD8 modes passed:

- Five new tests: exact matrix bits on repeated/changed/revisited bases and all
  current translations; 64 paired hierarchy/save passes; first component-error
  order; composed overflow before a later invalid component; and unchanged
  velocity/UI/physics domain refusals even with hit-eligible bases.
- Seven existing numeric-column tests, including actual storage, change observers,
  repeated mutable visits, successful unwind publication and partial-error rollback.
- Strict Clippy for the runtime library and tests.

That is 12 tests per mode, 24 executions total. The lean runtime/script dependency
guard, scoped format, generated conventions and diff checks also passed. All ten
commands used retained source inputs; native commands used Rust 1.99.0, explicit
`aarch64-apple-darwin`, CI profile, locked/offline dependencies, two jobs and Rayon 2.
No full runtime regression suite, shipping timing, GPU result or performance
acceptance is claimed. Four test executable copies were saved immediately with
mode 0755 before any target reuse, representing two distinct numerical-mode binaries.
The supervisor and all children are terminal/reaped; the builder was released.

## Evidence and next gate

`docs/spikes/evidence/numeric-basis-cache/` holds a lossless compact packet with
73 records, 69 portable payloads, all command logs, source freezes, pinned library
copies and the independent source receipt. Four executables remain externally
hash-bound. The verifier checks exact test filters/counts, commands, binary hashes,
source001-to002 delta and all 3,874 build inputs against the source commit:

```sh
python3 docs/spikes/evidence/numeric-basis-cache/verify.py --external --git-source
```

The next step is an ordinary source-free shipping export with exactly the existing
03bc Core workload, followed by full report/PCM/all-row parity and a separately
approved quiet comparison against retained baseline player `a2794edc`. No timing
or source intake is approved by this correctness packet alone.
