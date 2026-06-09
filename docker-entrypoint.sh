#!/bin/sh
set -e

CONFIG=/data/clef-note.toml
mkdir -p /data

if [ ! -f "$CONFIG" ]; then
    if [ -n "$PASSWORD_HASH" ]; then
        printf 'password = "%s"\n' "$PASSWORD_HASH" > "$CONFIG"
    elif [ -n "$PASSWORD" ]; then
        echo "Hashing password…"
        HASH=$(/app/clef-note --hash-password "$PASSWORD")
        printf 'password = "%s"\n' "$HASH" > "$CONFIG"
        echo "Password hash written to $CONFIG"
    else
        echo "ERROR: set a Fly secret before deploying:"
        echo "  fly secrets set PASSWORD=yourpassword"
        exit 1
    fi
fi

# Weekly demo reset — wipe notes if the marker is older than 7 days
RESET_MARKER=/data/.last_reset
if [ -f "$RESET_MARKER" ] && find "$RESET_MARKER" -mtime +6 | grep -q .; then
    echo "Weekly reset: clearing demo notes…"
    rm -rf /data/notes /data/assets /data/drawings
    touch "$RESET_MARKER"
elif [ ! -f "$RESET_MARKER" ]; then
    touch "$RESET_MARKER"
fi

export CLEF_NOTE_CONFIG="$CONFIG"
exec /app/clef-note --partitions /data --port "${PORT:-8080}"
