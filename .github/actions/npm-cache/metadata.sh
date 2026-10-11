set -euo pipefail

# Match setup-node: npm's configured directory, Node's lowercase architecture,
# and GitHub's lockfile hash. Do not cache node_modules or add prefix fallback.
[[ -f "$NPM_LOCK_FILE" && "$NPM_LOCK_HASH" =~ ^[a-f0-9]{64}$ ]] || {
  echo 'npm cache requires an existing lockfile and its exact hash' >&2
  exit 1
}
npm_cache_dir="$(npm config get cache)"
npm_cache_arch="$(node -p 'require("node:os").arch()')"
for npm_cache_value in "$npm_cache_dir" "$npm_cache_arch" "$RUNNER_OS"; do
  [[ -n "$npm_cache_value" && "$npm_cache_value" != *$'\n'* && "$npm_cache_value" != *$'\r'* ]] || {
    echo 'npm cache metadata must be a nonempty single line' >&2
    exit 1
  }
done
printf 'path=%s\nkey=node-cache-%s-%s-npm-%s\n' \
  "$npm_cache_dir" "$RUNNER_OS" "$npm_cache_arch" "$NPM_LOCK_HASH" >> "$GITHUB_OUTPUT"
