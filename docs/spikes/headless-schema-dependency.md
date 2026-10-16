# Normal headless coroutine schema compilation

Checkpoint49 at `62ee33ffd8b45b30450ca3d33cf96bfa9d1e6bbb` fails to compile the
ordinary headless binary on all three desktop hosts. The coroutine schema
registrations use `incant_types`, which was declared only as a dev-dependency.
Rust tests could access that dependency; the normal executable could not.
The [Mac general job](https://github.com/snowdamiz/incant/actions/runs/38830772451)
and [Linux/Windows desktop jobs](https://github.com/snowdamiz/incant/actions/runs/38830772417)
all report E0433 at the two registrations. Later missing-artifact errors follow
the failed build; they are not independent gameplay failures.

Move the existing dependency to the normal dependency table. No runtime code,
schema, feature selection, version resolution or Cargo.lock entry changes.
The existing full-suite ordinary-binary build remains the regression check.

On this Mac, the corrected ordinary `incant_headless` binary builds with the
locked `ci` profile, generates all 80 schemas byte-identically to the published
registry, and passes strict all-target headless Clippy. All seven schema-checker
controls, generated conventions, formatting and whitespace checks pass.
An independent review confirms the manifest scope and unchanged dependency
resolution. [Exact local results and failure identities](evidence/headless-schema-dependency.json).

The initial local build launcher could not find Cargo on PATH and launched no
compiler. The successful attempt explicitly selects the installed pinned
toolchain. An initial formatting invocation had the same tool discovery issue;
the explicit-toolchain invocation passes. Neither setup failure is discarded.
This qualification is local compilation and schema behavior, not a shipping
performance or phase-gate result. Checkpoint49 remains failed; release stays at
the previous verified baseline until a corrected frozen candidate passes every
required check.
