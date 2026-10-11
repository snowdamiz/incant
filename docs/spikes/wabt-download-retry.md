# Pinned WABT transport retries

Main80 Linux job `114404207093` (run `38117149408`) passed static
conformance and the standalone shipping proof, then the composed proof stopped
before its build with TLS `ConnectionResetError(104)` while downloading WABT.
The captured job log is `/tmp/incant-main80-Linux-wasm-failure.log`.

`tools/probes/shipping-wasm2c.py` now makes at most three download attempts for
connection/timeout failures and HTTP 408, 429, 500, 502, 503 or 504. The backoffs
are 0.5 and 1 second. Each attempt retains the existing 30-second urllib timeout;
this is not a total wall-clock deadline. Partial transfers are truncated before
the next transfer. The release URL, version, host selection, SHA256, 32 MiB
transfer cap and exact regular archive-member checks are unchanged. Certificate
validation, permanent HTTP errors, checksum/size/member failures and unrelated
errors fail without retry. Successful provenance includes the attempt count.

Five local regression tests exercise actual localhost HTTP disconnect/503
recovery, exhaustion after three attempts, permanent HTTP/checksum refusal,
certificate refusal, and interrupted-transfer truncation. Against the unchanged
helper they produced three errors and one failure; after the correction all
95 tools tests passed. The local fixture pins its own small test archive and
does not replace any production pin. These tests establish local fault behavior,
not a live provider reliability gate.

A separate real macOS arm64 download succeeded on the first attempt. Both
extracted executables reported `1.0.42`; the archive matched the unchanged pin
`3f654779b436c628db3ca4323c51538815f5fbc887a6d5f3bb1c18d36281e240`.
No compiler/runtime/vendor/workflow input changed. Hosted execution remains
separate from this local evidence.

| Evidence | SHA256 |
| --- | --- |
| `/tmp/incant-wabt-retry-before.log` | `5b748416371bfe91d2a1f1f2b41896dbba2c459ec327e513abdb0ab2e792af6f` |
| `/tmp/incant-wabt-retry-after.log` | `ebbff43a77b7e2e7ebd51691059bd84537679feb7c4c89cc9d069225bbdec64d` |
| `/tmp/incant-wabt-retry-live.json` | `5de4f0c4a7c4d79b669c5535cf6e8d6865a93174a1891eb53537ac57850ef4a4` |

Validation: `python3 -m unittest discover -s tools/tests`,
`python3 tools/generate_conventions.py --check`, and `git diff --check`.
