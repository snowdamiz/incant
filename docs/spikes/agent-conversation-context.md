# Native agent conversation context

`Agent::run_shared_in_conversation` continues an in-memory conversation bound to
one opened project's stable ID. It preserves completed tool exchanges across
turns while using a separate provenance identifier for each actual run. Existing
stateless APIs still use the same dispatch and provider loop. Nothing is written
to the project, transcript files, telemetry or credentials.

Retained context has a 16-turn, 2 MiB bound. Eviction removes whole turns, so it
cannot separate a function call from its output. Prior screenshot pixels become
an explicit recapture notice; metadata and opaque tool JSON are preserved.
Oversized turns and omitted older context are labelled explicitly. Failed turns
retain their user request and an incomplete-turn notice that warns that earlier
operations may have committed. They never replay an unmatched tool batch or
claim its accepted transactions were rolled back. Reusing history for another
project is refused until the caller clears it.

A bounded JSON counter caps the complete input/tool request, including framing
reserve, at 16 MiB before networking. Oversized requests return ContextLimit
without charging an unmade request. Existing actual response accounting and
conservative failed-provider reservations remain unchanged.

Validation: all 53 agent tests pass, including whole-turn eviction/image omission,
UTF-8 byte bounds, multi-turn protocol/provenance with a concurrent manual edit,
partial-batch interruption preserving an accepted transaction, and pre-network
context refusal. All-target Clippy with denied warnings and formatting pass.
Independent review accepted all three source/test identities.

The provider is a deterministic protocol double in these tests. This does not yet
prove a live provider's multi-turn acceptance or connect the editor chat. Native
session/controller, account and bridge integration remain separate work; the
caller serializes access to this history. No latency/device or phase-gate claim
is made. See [source and validation identities](evidence/agent-conversation-context.json).
