# Windows budget journal test lifetimes

Checkpoint6's Windows desktop job passed five budget tests and failed two at
independent journal reads. The owning command bus still held its exclusive file
lock; Windows returned OS error33. Production locking and budget behavior are
unchanged by this correction.

The tests now close the owner before reading exact journal bytes, reopen it, and
assert recovered project/revision/history/redo before continuing. Every rejected
operation still compares exact CRDT bytes immediately before and after on the
same bus. User-origin merge, Undo/Redo and final reopen checks remain. An initial
added cross-reopen byte comparison was invalid because equivalent restored CRDT
exports need not have identical serialized order; its failure is retained along
with the initial unused-mut lint finding.

Final seven budget tests and scoped Clippy pass. Hosted Windows confirmation
remains pending; checkpoint6 is not promoted. These are test-only changes, not a
reduction in admission, journal or transaction guarantees.

[Source and retained logs](evidence/checkpoint6-budget-journal-tests.json).

[Final independent source review](evidence/checkpoint6-budget-journal-root-review.json) accepts the corrected test scope.
