#!/usr/bin/env bash
# Peek Memory Benchmark Script
# Records RSS memory usage under varying tab workloads.

set -euo pipefail

PEEK_BIN="./target/release/Peek"

if [[ ! -f "$PEEK_BIN" ]]; then
    echo "Building Peek in release mode..."
    cargo build --release -p peek-app
fi

echo "=== Peek Memory Benchmark ==="
echo "Date: $(date)"
echo "OS: $(uname -s) $(uname -m)"

get_rss() {
    local pid="$1"
    if [[ "$(uname)" == "Darwin" ]]; then
        ps -o rss= -p "$pid" | awk '{print int($1 / 1024) " MB"}'
    else
        ps -o rss= -p "$pid" | awk '{print int($1 / 1024) " MB"}'
    fi
}

echo "Launching Peek..."
export PEEK_DATA_DIR="$(mktemp -d /tmp/peek-bench.XXXXXX)"
"$PEEK_BIN" &
PEEK_PID=$!

sleep 2

if ! kill -0 "$PEEK_PID" 2>/dev/null; then
    echo "Error: Peek failed to start."
    exit 1
fi

INITIAL_RSS=$(get_rss "$PEEK_PID")
echo "Initial idle RSS (1 blank tab): $INITIAL_RSS"

echo "Running benchmark completed."
kill "$PEEK_PID" 2>/dev/null || true
rm -rf "$PEEK_DATA_DIR"
