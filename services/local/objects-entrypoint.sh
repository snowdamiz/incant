#!/bin/sh
set -eu

# Linux bind mounts retain the host owner's UID. Keep that file private and
# read-only; give only the image's service user a private, ephemeral copy.
umask 077
cp /config/s3.json /run/incant-objects/s3.json
chmod 0700 /run/incant-objects
chmod 0600 /run/incant-objects/s3.json
chown seaweed:seaweed /run/incant-objects /run/incant-objects/s3.json

# The pinned image prepares /data and drops to seaweed before starting servers.
exec /entrypoint.sh "$@"
