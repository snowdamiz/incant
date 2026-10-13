# guard.sh: sourced by every 0060 build, run, capture and window tool.
# lease_guard refuses (exit 2, nothing created) after the release marker, without the
# 0060 lease, or once the lease's explicit expiry has passed. Tools never extend a lease.
S=$HOME/.codex/incant-evidence/0060
TARGET=$HOME/.codex/incant-native-target
lease_guard() {
  if [ -e "$S/released" ]; then
    echo "0060 lease was released; refusing" >&2
    exit 2
  fi
  /usr/bin/python3 -I -c '
import datetime, json, sys
try:
    lease = json.load(open(sys.argv[1], encoding="utf-8"))
    expiry = datetime.datetime.fromisoformat(lease["expires_at"])
except (OSError, ValueError, KeyError):
    sys.exit(1)
now = datetime.datetime.now(datetime.timezone.utc)
ok = (lease.get("packet") == "0060-core-sample-presentation" and now < expiry
      and lease.get("cargo_target_dir") == sys.argv[2])
sys.exit(0 if ok else 1)
' "$S/lease.json" "$TARGET" || {
    echo "no valid 0060 lease (missing, foreign or expired); refusing" >&2
    exit 2
  }
}
