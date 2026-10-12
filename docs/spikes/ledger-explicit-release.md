# Explicit ledger lock ownership release

Checkpoint7's general macOS job failed reopening a unique temporary ledger after
an invalid receipt was consumed: `LedgerError::Busy` surfaced at the next open.
The failed job is preserved. A local unchanged 62-test run passed, so it does not
reproduce that exact hosted scheduling interleaving.

The ledger relied on File close to release its Unix flock. A duplicated open file
description can outlive that File, including an inherited descriptor between
fork and exec. A deterministic actual fork/pipe-barrier model shows the lock
remains after the parent closes its descriptor; explicit unlock releases it while
the child still holds its copy. This is a concrete ownership-lifetime deficiency
consistent with the hosted failure, not proof of its exact scheduling cause.

A private guard now explicitly unlocks while its descriptor is valid, then closes
it. It is constructed immediately after successful lock acquisition, covering
record-load refusals as well as completed, invalid and abandoned leases. Lock
acquisition and accounting policy are unchanged. A live owner still excludes
other opens; invalid receipts and dropped reservations retain the full charge.

Two deterministic regressions retain a cloned descriptor. Both fail on original
source with Busy and pass after the correction. They cover ordinary drop,
abandoned reservation, invalid receipt, completed usage, and continued exclusion
by a replacement live owner after the old alias closes. All 64 agent tests and
all-target Clippy pass. No retry or test serialization is used. Unix alias tests
are not a substitute for the next actual hosted Windows/macOS/Linux checkpoint.

[Exact source, red/green logs and process model](evidence/ledger-explicit-release.json).

[Independent source review](evidence/ledger-explicit-release-review.json) accepted
the private owner boundary and deterministic accounting/exclusion regressions.
