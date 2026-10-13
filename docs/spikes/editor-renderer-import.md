# Shared renderer test helper import repair

Checkpoint18's Linux and Windows desktop jobs fail to compile the editor's
`model_gpu` test: the shared renderer test helper accepts `crate::Renderer`, but
the editor integration test had no such root import. The renderer's own tests
already supplied the type. The repair imports the existing renderer type in that
test root and passes its existing headless renderer through the unchanged checked
backend helper. Production code and backend matching rules are unchanged.

The affected editor test now compiles on macOS and runs its CPU backend contract:
one test passes and the GPU test remains ignored. The renderer library's same
CPU contract separately passes. Scoped editor Clippy, formatting, conventions and
diff checks pass. These six commands bind 2,029 frozen inputs. No GPU execution or
hosted Linux/Windows repair pass is claimed.

[Evidence](evidence/editor-renderer-import.json) preserves both exact failed hosted
logs, the small patch, source acceptance and successful local outputs. Both hosted
jobs report the same Rust E0425 compiler error; Linux's wrapper exits101 and the
Windows PowerShell wrapper exits1. An initial collector assumed both wrapper exit
codes were101; correcting that assertion changed no source or command result.

Checkpoint18 remains frozen and unpromoted while its other jobs finish. The next
reviewed candidate must pass all23 required contexts; a local compile does not
satisfy that release requirement. [Final integration review](evidence/editor-renderer-import/final-review.json) is accepted.
A subsequently completed macOS checkpoint job reports the same E0425; its exact
log is retained separately without rewriting the original two-job archive.
