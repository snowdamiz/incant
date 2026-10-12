# Native export startup environment

The exporter now omits HOME from its restricted startup subprocess environment.
It retains the player-only PATH, private temporary directories and existing
Windows platform keys. The parent environment is unchanged.

Eleven focused native-package/UI-export tests pass, including an actual POSIX
child process launched through the ordinary exporter startup route with only
the build stage replaced by a fixed executable fixture. That check establishes
subprocess environment behavior, not a compiled Incant player or device gate.
The [test log](evidence/export-startup-environment.log.gz) and
[independent acceptance](evidence/export-startup-environment-review.json) are retained.

Earlier export receipts remain historical evidence for the source versions that
created them; they are not rewritten to claim the new environment. New ordinary
public export proofs bind this corrected exporter. Windows fixture execution and
reference-device gates are not established by this local POSIX check.
