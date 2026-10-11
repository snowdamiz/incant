# Bounded native script logs

The native QuickJS context provides `log(message, level?)` using the same argument
order as the authoring SDK. A missing level means `info`; `debug`, `warn` and
`error` are also supported. Message and level must be primitive strings. No
string coercion, guest formatter, object traversal, console I/O, project access
or JSON conversion occurs in the callback.

A message accepts at most 1,024 UTF-8 bytes and may contain tabs/newlines but no
other control characters. An update accepts at most 64 messages and 16 KiB of
message text. Byte-length preflight stays in VM-managed storage; checked UTF-8
conversion rejects lone UTF-16 surrogates before Rust string inspection. The
same correction is applied to timer IDs in their parent change. Valid Unicode,
including surrogate-pair emoji, is preserved.

Logs stage with the synchronous update and commit after numeric validation,
timer validation and the complete native structural batch. A caught invalid call
still rejects the whole update. Exceptions, unsupported returns and rejected
structural commands publish no pending messages. Retained log functions expire
before diagnostics and cannot acquire authority on another update.

Each host retains a prefix of at most 1,024 records and 256 KiB of message text.
Once either bound is reached, later committed messages increment a saturating
`dropped` counter. A full history does not fail otherwise valid gameplay. Records
carry the accepted script clock, level and message. Standalone ScriptHost updates
do not step the world; NativeSession keeps the clocks aligned. Invalid input
packets and output buffers append nothing and remain retryable.

NativeSession and the device/offline/hosted game wrappers expose the committed
history by reference. A terminal script, simulation, audio or scene-host failure
hides every public observation, including previously committed logs. The
source-free game player's final JSON report includes `logs.entries` and
`logs.dropped`; serialization happens once after the completed run. Existing
failure and create-new output semantics remain in place.

The capability is QuickJS-only. It does not introduce a Wasm log import, migrate
the editor's legacy Play path, persist VM/session state or complete the native
SDK. This is bounded allocated storage, not an allocation-free or phone-budget
claim. Full Core Sample/device/release gates remain open.

Validation passed: 87 scoped script/game tests and doctests (including eight new
logging tests), 585 ordinary workspace tests/doctests, scoped and workspace
all-target release Clippy, the GPU player's optional-feature Clippy, 49 Python
tool tests, strict SDK TypeScript, and six normal-dependency guards. Independent
Astra review verified all 11 final source hashes after the Unicode correction.

The existing fresh shipping proof passed 47 process controls in CPU render-input
mode after deleting authoring and temporary build inputs. Existing motion, audio,
startup refusal and headless observation controls remain. Baseline runs preserve
Unicode text, levels and accepted ticks. Separate count/byte-history runs retain
1,024/256 records and drop exactly 6,656/1,664 while producing identical motion
and PCM. Four invalid/caught log-update runs emit no completed report or PCM.
CI retains the existing GPU proof command with these additive controls; this
change adds no fresh shipping build or job to the workflow.

The macOS arm64 CPU-mode player is 4,686,928 bytes, SHA-256
`cf78f85993ffb80cacabfb8642461ecc73cd62c0178962087b5324bf867cd00f`,
with dSYM UUID `d3d4c553-6783-37ee-8118-b2cf05beb5d2`. Its graph differs from the
earlier GPU player, so these sizes are not an isolated feature-cost comparison.
[Exact source, review, logs and process evidence](evidence/native-script-logs.json).
