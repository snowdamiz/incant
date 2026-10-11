# Captured Wasm preparation input

Static preparation now sends the captured `mN.original.wasm` file to the cook
validator/meter and hashes that same file for `input_sha256`. Previously it
captured the input, then cooked and hashed the mutable external path. Replacing
that path between these steps could translate different bytes or label the
prepared module with a different input digest; removing it could fail preparation
after capture. The change preserves the existing generated outputs and public
facade ABI.

Three deterministic file-race regressions replace the external input before the
cook reads it, replace it after cooking but before manifest hashing, and remove it
after capture. They observe the actual supplied input bytes, prepared bytes and
manifest digests through controlled tool boundaries. All three fail on the
main98 baseline and pass with the two-line correction.

Local checks on 2026-10-11: all 15 scoped Wasm tool tests and the complete 213-case
Python tools suite passed, with four existing skips. Generated conventions and
diff checks passed. A separate run used the existing pinned WABT 1.0.42 translator
and a private copy of the reviewed cook binary, after checking its cook/ABI source
against main98. Replacing the external input with invalid bytes immediately before
the real cook call still prepared the captured module and recorded its original
digest. No guest exports were executed. Logs and exact commands are retained in
`/tmp/incant-wasm2c-capture-proof`; this is local preparation evidence.

The separate static linked identity prototype remains frozen and unadopted. No
identity sidecar, runtime API, save guard or numerical compatibility change is part
of this correction. Implementing model: Sol 6.1 Extra High (`gpt-6.1-sol`, `xhigh`),
Astra.
