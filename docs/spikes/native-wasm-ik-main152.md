# Complete native IK SDK on main152

The reviewed IK-v1 Rust/Wasm API is integrated with actual main152
`1ab376c799ab61e179f15af8adc94db21188086c`. Tested source `c8115289` and the
current-main ancestry merge `fec80d13` differ only in documentation. All 1,727
frozen inputs and all 29 accepted producer source/test/helper files remain exact;
the producer conflict affected only STATUS. Operations 44–46 are additive and
carry full v4 poses plus ordered IK constraints with atomic controls. Prior
operations/layouts and the eight legacy pose refusals remain intact.

The current-source actual Wasmtime/wasm2c suite passes 418 tests in 74 groups,
zero failures or ignored tests, then all-target Clippy and three development
processes. The helper verified 841 prepared artifact files before cleanup. All
13 top-level commands pass, including wasm32, dependency guards, generated
contracts, tools, formatting and matching headless/oracle builds.

A fresh ordinary shipping export passed public import, atomic authoring,
Undo/Redo and journal reopen, then removed its private source/cache/build/tool
copies and debug maps. Three fresh copied-player processes each match 4,140
complete scalar observations and 252 public vector checks against the independent
native oracle and development host. PCM and logs match exactly. Fifteen failures
match their exact diagnostics with empty stdout and absent output. Fourteen actual
package files and 17 removed paths were verified. Player SHA256 is
`31403e3ec29c3c7ff1aa88fcdd45b7665e6569c5d711ae441ee9589629843226`
(9,869,136 bytes).

[The inventory](evidence/native-wasm-ik-main152.json) retains source identities,
all commands, compressed/raw logs, proof receipts, reports and PCM. The
[producer contract](native-wasm-ik.md) remains bounded: no GPU/appearance, device,
shipping-performance or phase acceptance; module checkpoints still refuse, and
nonzero compound graph poses require the separate complete-pose protocol.

Checkpoint12 remains frozen at older main150. Its actual Linux Vulkan and Windows
DX12 linear-RGBA8 reference failures are preserved independently of this feature;
Claude0059 owns their correction and release remains held.
