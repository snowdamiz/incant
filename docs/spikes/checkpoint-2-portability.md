# Release checkpoint 2 portability repairs

The frozen main88 snapshot `c3ec3e84` completed all eight workflows with 20 of
23 required contexts successful. Three failures were diagnosed and reproduced;
the checkpoint was not promoted and its results are retained unchanged.

- Linux local services: the pinned S3 server drops to UID1000 while a private
  runner-owned configuration belonged to UID1001. A private tmpfs copy keeps the
  original mode0600 and delegates to the pinned entrypoint before networking.
  The actual local Docker proof passes 28 protocol checks and four restart /
  persistence controls. Diagnostics expose only allowlisted service state.
- macOS general checks: Python3.12 lacks `os.waitid`. A Darwin `kqueue` fallback
  observes child exit without reaping the group leader before cleanup. Both
  actual Python3.9 and forced fallback on Python3.14 pass the lifecycle cases;
  Windows JobObject behavior and the existing waitid path remain unchanged.
- Windows desktop: Node rejected a canonical verbatim entry path before loading
  the trusted compiler. A fixed engine-relative entry and narrowly converted,
  canonical-equivalent launch directory preserve the scrubbed environment.
  Nine actual local authored compilation / native-motion / headless cases and
  scoped Clippy pass; hosted Windows execution is still required.

The integrated repair preserves all three reviewed commits. Its 199 Python tests
pass with four Windows-only skips; 17 Node tests, formatting, generated conventions
and actionlint pass. All Rust/Cargo bytes equal the independently reviewed Windows
repair, so its scoped Rust checks apply unchanged. No workflow, required check,
shipping profile, production deadline or assertion has been removed.

[Recorded identities and logs](evidence/checkpoint-2-portability.json) link the
integration to each component's evidence:
[Linux bootstrap](local-service-foundation.md#linux-private-bootstrap-correction),
[Darwin observation](darwin-process-observation.md), and
[Windows compiler entry](windows-native-compiler-entry.md).

Actual hosted Linux AMD64, macOS Python3.12 and Windows confirmation remains for
the next frozen checkpoint after the repair lands on main. Local execution is
not a device/performance or release-promotion claim.
