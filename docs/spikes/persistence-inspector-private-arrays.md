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

This is a source candidate awaiting native qualification and independent review.
There is no shipping performance or 2 ms gate claim. Cursor sample changes remain
a separate commit and separate evidence packet.
