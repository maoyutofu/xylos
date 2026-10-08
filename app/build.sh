#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
DIST_DIR="$SCRIPT_DIR/dist"

if ! command -v dx >/dev/null 2>&1; then
    echo "error: dioxus CLI (dx) is required" >&2
    echo "install it with: cargo install dioxus-cli" >&2
    exit 1
fi

echo "Building Dioxus Web client..."
# Dioxus may retain hashed assets from an earlier build in its output directory.
rm -rf "$SCRIPT_DIR"/target/dx/*/release/web/public
(cd "$SCRIPT_DIR" && dx build --platform web --release)

PUBLIC_DIR="$(find "$SCRIPT_DIR/target/dx" -type d -path '*/release/web/public' -print -quit 2>/dev/null || true)"
if [[ -z "$PUBLIC_DIR" || ! -f "$PUBLIC_DIR/index.html" ]]; then
    echo "error: Dioxus build completed without a Web public directory" >&2
    exit 1
fi

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"
cp -R "$PUBLIC_DIR"/. "$DIST_DIR"/

echo "Static files copied to: $DIST_DIR"
echo "Start Xylos from the project root with:"
echo "  cargo run -- --config config.toml"
echo "Then open: http://127.0.0.1:8080/app/"
