# Incant original-history backup

This branch preserves the exact unpublished history of
`codex/integrate-core-authored-main182` without rewriting its commits.
GitHub rejected the original branch because its evidence archive exceeds the
normal Git file-size limit. The bundle is stored using Git LFS.

`manifest.json` records the original head, bundle checksum, and two prerequisite
commits. Both prerequisites are in branches pushed to this GitHub repository.

## Restore

In a clone of `snowdamiz/incant` with Git LFS installed:

```sh
git fetch origin '+refs/heads/*:refs/remotes/origin/*'
git switch --detach origin/backup/2026-11-07-large-history
git lfs pull
git bundle verify incant-core-main182.bundle
git fetch ./incant-core-main182.bundle refs/heads/codex/integrate-core-authored-main182:refs/heads/restored/core-main182
git rev-parse restored/core-main182
```

The restored head must be `f8cc8d87c778f0c0c9a805575c3cfaafa8758b15`.
The original history contains unqualified work; this is a backup, not a release
or test qualification.
