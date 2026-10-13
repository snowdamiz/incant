# Authored iOS Wasm on main185

The current integration preserves Android TypeScript admission and explicitly
refuses Android static modules before processing their bundle or publishing
output. The real cooker regression covers this composition boundary. Eleven of
thirteen producer source files are byte-identical; the two cooker files contain
the reviewed Android admission and refusal-test integration.

Frozen `4b277d577c6627e98e407b2c25daf274a99fd406` contains actual main185.
All 2,001 frozen inputs remain unchanged. The cooker passes 62 tests with no
failures or ignored cases; all-target Clippy, focused exporter/module tests,
dependency checks, formatting and conventions pass. An initial test-discovery
pattern selected no files and exited 5; its log remains retained. The actual
test modules then passed with no source changes. There are 21 successful
commands and this one historical harness failure.

Both physics modes were freshly authored, cooked and exported through the normal
iOS path into shipping simulator archives. Twelve source/map/build-deleted C and
Swift processes match all 96 native-oracle ticks, 186 scalars per tick, 248 selected
vectors and exact nonzero PCM. Fifty-two exact refusal controls pass, including
reciprocal wrong-mode packages. Compiler commands, five actual solver feature
identities, static C compilation, archive contents, final linked symbols and
owned simulator shutdown/deletion are re-evaluated by the retained collector.

Current execution is simulator CPU only. Device unsigned links remain the
separately accepted original `a7654dd9` producer evidence, including its original
compiler-recovery qualification. Current simulator archives were both fresh
normal exports. No physical-device, minimum-OS, performance, rendering, signing
or phase approval is implied.

Final current-main independent review is pending. The [manifest](evidence/authored-ios-wasm-main185.json)
binds current raw streams, source inventories, retained assertions, actual output
and external binary hashes. Lossless archives use bounded 16 MiB parts.
