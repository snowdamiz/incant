# Bounded provider response reads

The OpenAI SSE reader previously checked its 32 MiB response limit after
`BufRead::lines()` had allocated a complete line. A malformed response could
therefore allocate an arbitrarily large unterminated line before that check.
Newline bytes also did not count toward the limit.

The parser now limits each read to the remaining response budget plus one byte
for overflow detection. Delimiters and comments consume the same budget. An
oversized or invalid UTF-8 response fails with a static diagnostic, without
echoing its contents. The limit bounds consumed response bytes; it is not a total
process-memory claim, since parsing, buffered transport and completed output
objects have their own allocations.

Four new controls check an oversized unterminated line stops at byte 65 of a
64-byte test budget, blank/comment byte accounting, exact-limit CRLF and multiline
UTF-8 completion, and redacted invalid UTF-8 refusal. The same parser path uses
32 MiB in production. Nine provider unit controls and all eight agent integration
controls pass, including terminal-success-only tool execution, incomplete-response
budget accounting, account invalidation and explicit credential selection.
Strict all-target Clippy also passes.

[Source and local verification receipt](evidence/provider-stream-bound.json)
records the producing base, exact changed source hashes and test artifacts. The
HTTP controls use local synthetic responses. No live account, provider expansion,
rendering result or Phase 3 gate is claimed.
