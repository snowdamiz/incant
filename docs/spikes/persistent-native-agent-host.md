# Persistent native agent host

The editor previously created its native Play host on each agent turn. A later
`play_state_query` therefore lost the preceding successful run despite the tool
contract. The editor now lazily starts one dedicated native owner. Existing
`incant_agent::Session` workers remain per-turn waiters and carry the same bounded
turn context, cancellation, approvals and accounting through a capacity-one
channel. Only owned requests and outcomes cross that channel. The host, its
QuickJS VM, renderer cache and conversation are constructed, used and dropped on
the owner thread; there is no unsafe Send implementation.

Retained scope includes the opaque account selection receipt, exact bus Arc,
project ID and trusted root spelling. A changed scope discards both the previous
host and conversation before its first request. Ordinary authoring revision
changes preserve the last accepted native world. A short project identity check
also runs at provider/tool fences, closing the preparation/capture race without
holding project locks across provider IO or overlapping project/account-store locks. Each turn still gets
its own provenance ULID. The native host's existing checked cook/publication path
is unchanged: a rejected candidate leaves the previous accepted world available;
a completed new run replaces it.

Closing the service cancels the active Session and disconnects the owner even
when client clones survive. Idle owners exit; blocked provider work remains
cooperatively cancellable and is never joined on the event thread. Approval
waiters wake on cancellation. Cancellation before enqueue is known not to have
executed work and remains uncharged. If an accepted owner request panics or loses
its reply, the Session waiter joins failed-owner teardown then uses Session's
existing panic path: uncertain usage exhausts the remaining allowance. No silent
owner restart can forget an unaccounted request. The channel is bounded, and
single-flight admission still lasts through the actual Session waiter exit.

## Local validation

On macOS, Rust's locked `ci` profile with `-j2` passed 117 tests across 14 result
groups, with one pre-existing GPU test ignored. This preserves all 109 existing
agent/native-host/editor checks and adds eight observable regressions:

- Actual compiled fallback/native world at tick 5 is queried in a later turn;
  a disconnected-gamepad candidate fails while preserving that state; a new run
  replaces it with tick 2 and different position. Project/source/journal bytes,
  document, revision and history remain unchanged.
- Account, project ID, exact bus and root changes cannot query the old world.
- Idle owner exit disconnects retained clients; owner panic terminates before
  the Session releases physical ownership and marks accounting uncertain.
- Blocked work keeps the Session busy while shutdown returns without joining;
  late text is rejected and approvals wake on shutdown.
- Shutdown before enqueue creates no owner and charges no work.
- A project switch during the receipt check refuses the old host before any
  provider request or tool.

Four-package all-target Clippy (`incant_agent`, `incant_agent_native`,
`incant_headless`, `incant_editor`), formatting, conventions and normal runtime
dependency guards passed. Independent review accepted all five implementation
and test source hashes with no actionable finding. Exact hashes and logs are in
[evidence/persistent-native-agent-host.json](evidence/persistent-native-agent-host.json).
The first test attempt had one fixture error: recording offset zero was rejected
before the intended gamepad failure. Offset one exercises the actual candidate
runtime rejection; the final complete run passed.

No GPU, screenshot, real provider/credential or native-window operation was
performed for this correction. It does not claim a performance or phase gate.
The provider-hint transport, Claude UI and live native account probe remain
separate integration work. Nothing changes the shipped runtime graph or the
editor's manual Play service.
