# Exact npm cache producer refs

The reviewed local follow-up prevents new PR-private npm archives from competing
with shared Rust dependencies. Five workflows use one small composite action:
nondefault refs invoke `actions/cache/restore`, while the exact repository default
branch and literal `refs/heads/codex/release-candidate` invoke the standard cache
action. The latter is reserved for the separately reviewed frozen release
checkpoint workflow; this local change does not create or update that branch. Its pinned post step runs on success
and does not upload again after an exact hit. All existing install, build, test,
probe, job identity and runner commands remain unchanged. Windows desktop keeps
its existing no-npm-cache policy.

The key retains setup-node's `node-cache-<RUNNER_OS>-<Node os.arch()>-npm-<hashFiles>`
namespace and `npm config get cache` path. Root and website lockfiles remain
independent. There is no `node_modules` archive or prefix fallback. Metadata runs
at the repository root even when a calling job defaults to `website`; values pass
through environment variables and multiline output injection is refused. The
website setup-node v6 automatic caching option is explicitly disabled, preventing
a future `packageManager` field from bypassing the policy.

The pinned implementations were checked directly: [setup-node key construction](https://github.com/actions/setup-node/blob/49933ea5288caeca8642d1e84afbd3f7d6820020/src/cache-restore.ts),
[restore-only action](https://github.com/actions/cache/blob/55cc8345863c7cc4c66a329aec7e433d2d1c52a9/restore/action.yml),
[success-only post step](https://github.com/actions/cache/blob/55cc8345863c7cc4c66a329aec7e433d2d1c52a9/action.yml)
and [exact-hit save refusal](https://github.com/actions/cache/blob/55cc8345863c7cc4c66a329aec7e433d2d1c52a9/src/saveImpl.ts).

Six new tests execute the actual metadata script against controlled npm/Node
outputs and exercise the actual action conditions for default, feature, PR, tag
and missing refs. The actual Bash decision is case-sensitive: `MAIN`,
`Release-Candidate`, a candidate suffix, empty and PR/tag refs cannot save. GitHub
expression string comparisons ignore case, so the action consumes only the
script's fixed `true`/`false` output. Nested lockfiles, lowercase architecture, spaces, missing
files, invalid hashes and output injection are covered. All 131 Python tooling
tests pass; actionlint and diff checks pass. The five workflow run-command bodies
are byte-identical to the frozen base. The metadata tests are Unix-only because
none of these cache callers runs on Windows; policy tests run on every host.
[Retained local evidence](evidence/npm-cache-producers-local.json).

Actual hosted cache-version compatibility and saves/restores remain unverified
until natural main/PR consumers run this action. Existing scoped cleanup freed
509,846,766 bytes from two obsolete merged-PR79/80 entries; that historical result
is not a measurement of this follow-up. No cache deletion, extra seed run, coverage
reduction or speedup is implied by this change.
