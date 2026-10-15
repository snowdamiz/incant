# Save and load

## Workflow

1. Identify the trusted host's versioned native session checkpoint capability and its supported components. Separate authoring save/undo from gameplay save/restore. Do not use project JSON, retained typed-array views or raw VM memory as a game save.
2. Declare each script's persistence version and store only bounded supported values through context.state. Include mechanic state that affects continuation; timers, physics, animation, audio and structural state require the native aggregate contract, not handwritten partial reconstruction.
3. Validate package/engine/script compatibility and integrity before replacing the live session. A failed load must preserve the current session; a failed write must preserve the last valid save. Host-owned paths and atomic publication stay outside agent/gameplay authority.
4. Define explicit migration/refusal policy, slot identity and restart/continue UX. Never bypass incompatible-source refusal or invent missing save data.

## Verification

Save at active gameplay, restore into a fresh host and compare a deterministic continuation, including relevant audio/HUD/timers/physics. Test corrupt/truncated data, changed scripts/package, quota exhaustion, failed publication and repeated restore. If only a subset is supported, state that subset and keep full save acceptance open.
