# Native Play integration on main81

The reviewed TypeScript cooker, retained native RPC and headless agent Play
candidate `fef07d9` is integrated with verified main81 `118a417`, preserving both
histories. The planar save reader correction `8369776` is included: unsupported
2D solver state is refused before a world can be installed. The tested source
is `a407958e63d4473065452e17c03a532c019bccf9`; final evidence edits do not change
production, dependency, workflow or test inputs.

All 695 ordinary workspace tests/doctests and 73 serialized GPU regressions
pass on this union, including both headless camera/frame-capture tests affected
by lazy GPU initialization. Workspace all-target CI-profile Clippy, 98 Python
and 12 Node tool tests, 536 UI tests, strict production UI build, all 58 generated
schemas, SDK/bridge/conventions/formatting, actionlint and the normal-runtime
dependency guard pass. This is functional validation, not shipping performance.

The rebuilt TypeScript developer player passes twelve actual source-deleted
process controls: six CLI, four retained RPC and two agent-capability RPC cases.
They preserve custom motion, Unicode logs and authoring data, inspect completed
false assertions, and retain the previous completed session after a failed
candidate. This uses a private warm developer target, not a fresh shipping target.
The earlier real saved-OAuth Astra invocation remains evidence for the unchanged
agent implementation; it was not repeated or counted as an integrated test.

[Exact source, test identities, process result and log hashes](evidence/native-agent-integration-local.json)
record these local results. Hosted checks remain required before merging.
Editor controls, authored Script attachment, native gameplay capture, complete
session/physics saves, device budgets and phase gates remain open.
