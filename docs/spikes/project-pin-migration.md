# Host-bound project pin migration

This slice applies only the already-reviewed schema-1 legacy-unpinned → exact
current engine metadata pin. It does not convert an older format or force an
incompatible engine range. `Project::empty` and old unpinned canonical bytes keep
the accepted compatibility contract. No CLI/RPC/application endpoint is added.
Independent source review and the bounded Mac native qualification pass.
Application routing and its end-to-end qualification remain pending.

The host constructs `CommandBus::open_project_file` from its opened project path.
The file capability has no serde/schema/Clone implementation. The journal sibling
is derived with the existing `.journal` extension. Both leaf names are validated
portable host names; all other names are fixed or derived from a canonical ULID.
Neither a command, project, journal record, review response nor marker can select
a document or backup path. The journal is opened no-follow, single-link and
exclusively locked. The same pinned parent directory and root source-transaction
lock cover project and script writes. Existing script paths remain `.ts` only.

One bounded review is retained in the bus. Its ID binds exact source bytes,
physical file identity and document revision. Open handles pin reviewed/admitted
nodes until the authority retires, preventing inode reuse after unlink. The bus
owns the locked journal writer; the file capability also holds a duplicate
journal-node handle. Both are retained for the opened session. A second review
retires the first.
Apply rechecks identity/text/revision/compatibility, then submits exactly one
`SetEngineVersion { requirement: =current }` through `incant_cmd`. The transaction
retains exact original text and canonical candidate; undo and redo use the same
file protocol. Caller-supplied review contents are never applied. Ordinary
metadata commands and old save APIs do not acquire this file authority.

## Bounds and versions

Each before/after text is at most 32 MiB. Encoded project markers and individual
journal records are at most 128 MiB, including JSON escaping. The host-bound
journal is at most 256 MiB. Encoding stops before exceeding its limit; reads are
bounded. Prepare reserves capacity for the larger of Commit or Abort, preventing
an otherwise-valid installation from consuming its recovery record's capacity.
These are explicit conservative admission limits, not a peak resident-memory
claim. This slice does not compact journals that reach the cap.

ProjectPrepare/ProjectCommit/ProjectAbort are journal protocol version 3. Existing
ordinary/script records keep versions 1/2 and their encoding. Legacy openers
refuse v3; the new opener reads all supported records. The exact Initialize,
sequence, chain, every logical CRDT projection, project identity, and pin-only
preimage/postimage joins must pass before any recovery decision or torn-tail
truncation. Digests establish correspondence, not trust in project data. An absent
project can recover only from a fully joined pending operation. Malformed,
future-version, wrong-identity or foreign-node state fails closed. The new host
route also refuses a nonempty journal lacking a complete Initialize; it does not
truncate an unrecognized first record. The legacy opener retains its prior torn-
first-record behavior.

## Ordering and recovery

1. Stage the document/CRDT; validate the retained review and journal capacity.
2. Create and sync an exact-byte, create-only backup. Create and sync the new
   file. Publish a bounded marker with their physical identities.
3. Append and sync ProjectPrepare.
4. Move the actual document to the operation's `previous` name without replacing
   anything. Recheck its bytes and node identity. Publish the new node with a
   no-clobber move; verify it and sync the affected directories.
5. Append and sync ProjectCommit. Only then install the logical document, revision
   and history. Publish the source receipt and retire the marker.

A known failed Prepare leaves the original document and no logical transaction.
A known failed install/Commit restores only the exact owned node, records Abort,
and does not advance logical state. Ambiguous journal durability poisons the
live bus and requires reopen. A durable Commit remains authoritative if cleanup
fails; subsequent operations refuse until recovery.

Before-Prepare markers can only be cleaned when the original named node matches.
An unfinished Prepare must match the marker operation exactly, including when
a different marker is otherwise structurally valid. No unmatched marker is
retired and no tail is truncated on that refusal. Uncommitted prepared operations
roll back; committed operations verify the
candidate then clean up; aborted operations verify the original. Backups remain
immutable attempt artifacts, not proof of committed migration. Unexpected
symlinks, hardlinks, collisions, missing required artifacts, replacements or
foreign bytes are preserved and refuse recovery. No recovery synthesizes a new
project from arbitrary backup paths. External editors do not participate in the
root lock: this is not a universal filesystem CAS against a writer retaining an
open handle; displaced nodes are retained and all observable mismatches refuse.

The pre-build control plan covered real fresh-process exits through apply/undo/redo,
partial OS write failures and ambiguous durability, missing project recovery,
foreign replacements, review invalidation, bounds, corruption, backup collisions,
and legacy script protocol replay. Source003 subsequently passed the qualified
controls and independent protocol review described below. Application wiring
remains a separate, unimplemented follow-up.

## Pre-build source review corrections

Source001 remains retained. Source002 closes the independent node-lifetime
finding with held file handles, closes the unmatched-Prepared/foreign-marker
state transition before filesystem effects, and applies the stricter new-host
incomplete-Initialize refusal. Added controls directly exercise unlink/recreate,
held unlinked nodes, a renamed-artifact valid foreign marker, and unchanged full
artifact inventories after refusal. At that source002 review checkpoint, no builds
had run and no application endpoints existed; the later execution follows below.

Source002 compiled. Its command unit run passed 19 cases and retained one real
redo/known-Commit-error recovery failure: an Abort re-export of an unchanged
imported Loro state did not byte-match the prior durable snapshot. An isolated
64-sample replay against the actual compiled document crate observed 14 such
encoding differences. Source003 retains the exact last durable logical encoded
snapshot in bounded journals and uses it for ProjectAbort and its capacity
reservation. Prepare and failed writes never advance that checkpoint. Replay
still requires exact bytes, including a negative control with the same project
projection but a separately created causal snapshot. Legacy v1/v2 writer bytes
are unchanged. Source003 qualification passed; the failed source002 attempt is retained.

## Actual bounded qualification

Rust 1.99.0, explicit `aarch64-apple-darwin`, locked/offline CI profile and two
Cargo/Rayon workers were used. Source003 passed 21 command and 15 source unit
controls, then the full two-crate suite: **87 passed, zero failed or ignored**
across 19 nonempty harnesses. Strict all-target Clippy, repository formatting,
conventions and diff checks passed. The process-restart and partial-write tests
launch real child executables and check apply/undo/redo, exact original bytes,
logical history, journal and file preservation. The 24 crash-boundary and 12
write-error combinations supplement existing script-source process controls.
All supervisors/children are reaped and the font target is released.

The original source002 19-pass/1-fail run is preserved. The retained-binary
three-process diagnostic happened to re-export identical bytes; the separate
compiled-document 64-sample diagnostic established the noncanonical encoding
case. Its first standalone compiler invocation omitted the host proc-macro
search path (E0463), then the exact-source corrected invocation passed. These
are diagnostic observations, not performance measurements or independent Loro
causal equivalence proofs. The repaired writer uses exact durable bytes and the
scanner still rejects a valid same-project snapshot with different causal state.

Portable evidence: `evidence/project-pin-migration/{manifest.json,records.json.gz}`;
`collect.py --root <checkout> --verify-only --originals --local-binaries` replays
all retained identities and assertions without executing the binaries. Existing
Transaction Rust literals are all inside `incant_cmd`; no exported command or
Project schema changed in this follow-up. No CLI/RPC schema/tool, older-format
conversion, GUI, GPU, performance or phase-gate acceptance is claimed. This is
the qualified host protocol for a later reviewed application binding, not a
claim that users can already invoke file migration through the editor/headless app.

## Current-main successor

The producer executions above remain source-specific. The later main273
composition and Windows-safe test observation successor are recorded separately
in [the integration report](project-pin-main273.md); no original execution or
archive has been rewritten.
