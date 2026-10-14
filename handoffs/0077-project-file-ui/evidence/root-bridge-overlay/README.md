# Root-owned bridge integration overlay

Root/Astra supplied these four files for the UI consumer to use. Claude did not author this patch and must keep these production bridge paths read-only. The separate patch is relative to the recorded source HEAD; each original file matches the handoff baseline. Root retains ownership of qualification and integration.

`BridgeResult.outcomeUnknown?: true` is local result metadata. The UI consumes it for truthful uncertainty and sets it on Shell promise rejection. It does not change Rust IPC/schema and does not establish native file persistence or a phase gate.

Built-by: astra
