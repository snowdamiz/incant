# Shared agent project access

The provider loop now supports a shared editor command bus through
`Agent::run_shared`. HTTP requests, text callbacks and approval callbacks do not
own its mutex. The existing exclusively borrowed `Agent::run` uses the same
dispatch and loop implementation. There is no second mutation implementation.

Document and source edits still execute through `incant_cmd`, with the supplied
revision, source precondition, actor and transaction history. An editor change
while the provider is thinking makes an old patch fail; the agent must query and
retry. Import preparation releases the bus, and the existing prepared-import
commit checks the project identity, revision and complete original document.
Screenshot work receives one owned authoring snapshot outside the lock.

Native Play needs a stronger boundary than merely cloning the document. A source
transaction can finish between that clone and compiler startup. Hosts used by a
shared agent therefore must explicitly support `play_run_checked`; unsupported
hosts neither advertise nor execute shared Play. The concrete headless host
finishes immutable scene, script and asset preparation, then calls the supplied
validator before the first candidate tick. The validator checks the original
project session, revision, source recovery state and cancellation under the bus
lock. A stale candidate is dropped and any previously completed session remains
available. Subsequent editor edits cannot change the validated cooked inputs.

Cancellation is checked before and after acquiring shared access. Poisoned access
and a different project identity end the turn without further tool execution.
The shared mutex must remain bound to one project session; replacing it with a
new bus that reuses the same project ID is outside this API's contract.

## Validation

- All 47 `incant_agent` tests pass, including seven new shared-access cases.
  A real second thread edits while a scripted provider waits; streaming and
  approval callbacks can access the bus; stale agent edits fail before a fresh
  revision succeeds. The accepted transaction keeps agent provenance and undoes
  without removing the user's edit.
- Shared asset import and source-write tools use a real persistent journal.
  Reopen and undo preserve their source bytes, asset identity and provenance;
  an unchanged reimport adds no transaction.
- Five native agent Play tests pass. The new regression performs a real
  `WriteScript` transaction between the scene snapshot and concrete native cook,
  rejects the mixed candidate, retains the prior movement result, and then
  accepts the new script from a fresh snapshot. Both scripts are actually
  typechecked, cooked and executed.
- Scoped all-target Clippy with denied warnings and formatting pass.
- After rebasing on main91, all five native cases and all 19 Node compiler
  cases pass with the integrated Windows compiler-path correction.
- Independent review found the document/source race above; the corrected checked
  boundary and regression were reviewed again and accepted.

Machine-readable source and log identities are in
[shared-agent-project-access.json](evidence/shared-agent-project-access.json).

## Editor loading container

`Agent::run_shared` also accepts `Mutex<T: ProjectSlot>`. `CommandBus` itself
implements the slot, retaining existing callers. The editor's `LoadState` returns
the exact bus in its `Ready` variant and refuses `Loading` and `Failed` states.
It neither clones the command bus nor creates a second journal or edit path.
The private shared-access adapter still releases the guard before provider,
approval and host callbacks, then rechecks cancellation and project identity on
reentry. The one-project-session-per-mutex contract above is unchanged.

The container regression refuses loading before any provider call, applies a
real agent transaction, undoes it through the same container, then invalidates
the container during the next provider response and refuses its pending patch.
All 48 agent tests and three native editor loading/recovery tests pass, along
with two-package all-target Clippy and formatting. Independent review accepted
all four changed source files. [Source and validation identities](evidence/editor-agent-project-slot.json).
This adapter is preparation for the native chat service; it does not connect the
chat panel or demonstrate a live editor agent turn.

## Limits

This is the agent-loop/backend portion of the editor connection. It does not
connect the chat panel or implement its approval UI, model chooser, turn history,
cost display or project budget settings. Claude still owns their visual work.
Provider fixtures test the real loop, not live-provider availability or a phase
gate. This change makes no latency or device-performance claim.

Source reads, atomic writes and journal synchronization still hold the bus for
their bounded command operation. Cancellation does not preempt a blocking network
read or the fixed compiler process; it prevents subsequent command execution and
candidate publication at the checked boundaries. The existing compiler's source
epoch checks are retained; this is not a compare-and-swap guarantee against
arbitrary external filesystem editors.

## Actual main92 integration

Merge `524ea5a` preserves original reviewed `d50fb0b` and actual main92
`22f30bd`, including navigation queries and the Windows compiler-host repair.
The merge is clean and all nine reviewed source hashes remain exact.

Both affected package suites pass **95 tests**, including every shared-access
case and the actual post-snapshot source-write regression. The two unchanged
headless GPU cases remain ignored in this CPU integration run. All-target Clippy,
a headless rebuild, all62 exact freshly generated contracts, SDK/bridge checks,
formatting and conventions pass. The rebuilt public host also passes32 source
transaction/Play RPC requests in two fresh processes, preserving native motion,
Undo/Redo, journal recovery and authoring bytes. The ledger above records the
source/proof/log hashes. Comprehensive platform validation remains assigned to
the next frozen checkpoint; this does not connect the editor chat panel.
