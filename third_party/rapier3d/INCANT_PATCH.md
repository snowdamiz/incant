# Incant Rapier 0.36.0 patch

`INCANT_UPSTREAM.json` hashes every original registry file. Preserve the upstream
license text copied from its published VCS commit, recorded in
`INCANT_LICENSE.json`; the upstream registry archives omit that file. The pinned tree has no root or Rapier kernel NOTICE file.
The only code addition is the default-off `incant-checkpoint` feature:
bounded trusted capture/install of serde-skipped persistent-island sleep scans.
All solver algorithms, serde formats, default features and version stay unchanged.
This adapter is diagnostic infrastructure, not hostile save admission. It requires
an independently matched completed native owner; shape, manager and journal
validation is not provided by a sleep-scan stamp check.
