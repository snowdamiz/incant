# Private persistent-state inspection arrays

Candidate scope: the trusted JavaScript persistent-state inspector returns temporary
arrays to Rust's typed parser. Each container previously appended its entries with
`Object.defineProperty`; objects required three such calls per entry. The candidate
sets the outer temporary array's prototype to null with a captured intrinsic and
uses direct indexed writes. Object key/value pairs use literal arrays with their
two own slots. Neither temporary array escapes to guest code.

The inspector still captures validation intrinsics before guest startup and checks
the exact original input prototypes, own descriptors, enumerable data fields,
string keys, array density and node preflight. Native parsing still applies the
same depth, byte, node, finite-number and transfer budgets. Public `state.get()`
values still use the existing frozen exposure path; public object/array prototypes,
scalar bits and fresh-copy behavior are unchanged. No cache or persistent VM root
is introduced. Guest proxy traps retain their existing call and transaction rules.

Four focused controls cover poisoned intrinsics plus Array.prototype index setters
and Object.prototype descriptor fields; exact frozen output and negative-zero bits
with independently counted cost; reentrant proxy state writes that replace captured
intrinsics; nested caught failure rolling back numeric and persistent state; and
seven additional invalid descriptor/prototype/proxy shapes. Existing persistence
quota, sparse/symbol/accessor, opt-in and stale-capability controls remain applicable.

Source `9b0a819d99541a0acc2f0f2431ffdeed69528aa6` passed independent review
`01a4637537547fc3960c01b84dc18458f58eadce3d026a16ec547a975089ffcc`.
On the explicit aarch64-apple-darwin CI target, nine focused persistence tests
passed, followed by all 217 script tests in each deterministic and SIMD8 mode.
Each full run includes 148 library and 69 integration cases (plus one empty profile
harness). The focused tests overlap those full runs. Strict all-target Clippy in
both modes, scoped formatting, generated conventions and diff checks all passed.
No production or fixture correction was required after the source freeze.

The portable packet retains the complete command logs, source freeze, immutable
build-input inventory and 19 immediately copied executable identities. Large native
executables stay externally hash-bound instead of being duplicated in Git:

```sh
python3 docs/spikes/evidence/persistence-inspector/verify.py
# Producer-host source objects, raw originals and all copied executables:
python3 docs/spikes/evidence/persistence-inspector/verify.py --external --originals --git-source
```

The supervisor and all recorded children are terminal/reaped; the CPU target is
released. There is no shipping performance or 2 ms gate claim. Cursor sample
changes remain a separate commit and separate evidence packet. This hardening
change is already covered by the existing script compatibility build identity and
requires a fresh trusted cook before any shipping comparison.
