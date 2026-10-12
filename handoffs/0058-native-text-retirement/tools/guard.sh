# guard.sh: sourced by every 0058 build, run, capture, probe and event tool.
# lease_guard refuses (exit 2, nothing created) after the release marker, without the 0058
# lease, or once the lease's explicit expiry has passed. An expired lease is not extended
# by a tool; driving a window past expiry is refused like driving it after release.
S=$HOME/.codex/incant-evidence/0058
lease_guard() {
  if [ -e "$S/released" ]; then
    echo "0058 lease was released; refusing" >&2
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
sys.exit(0 if lease.get("packet") == "0058-native-text-retirement" and now < expiry else 1)
' "$S/lease.json" || {
    echo "no valid 0058 lease (missing, foreign or expired); refusing" >&2
    exit 2
  }
}
