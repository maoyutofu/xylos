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

# Ensure manifest and icons from public/ are copied to dist/
cp -f "$SCRIPT_DIR"/public/manifest.webmanifest "$DIST_DIR"/ || true
mkdir -p "$DIST_DIR"/assets
cp -f "$SCRIPT_DIR"/public/assets/* "$DIST_DIR"/assets/ || true

# Check if index.html in dist has manifest link, if not inject it before </head>
if ! grep -q "manifest.webmanifest" "$DIST_DIR/index.html"; then
    echo "Injecting PWA manifest and mobile meta tags into dist/index.html..."
    PWA_HEAD='<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover"><link rel="manifest" href="/app/manifest.webmanifest"><meta name="theme-color" content="#ffffff"><meta name="apple-mobile-web-app-capable" content="yes"><meta name="apple-mobile-web-app-status-bar-style" content="default"><meta name="apple-mobile-web-app-title" content="Xylos"><meta name="mobile-web-app-capable" content="yes"><link rel="apple-touch-icon" href="/app/assets/apple-touch-icon.png"><link rel="apple-touch-icon" sizes="180x180" href="/app/assets/apple-touch-icon.png"><link rel="icon" type="image/png" sizes="192x192" href="/app/assets/icon-192.png"><link rel="icon" type="image/svg+xml" href="/app/assets/icon.svg">'
    sed -i '' -e "s|</head>|${PWA_HEAD}</head>|g" "$DIST_DIR/index.html"
fi

echo "Static files copied to: $DIST_DIR"
echo "Start Xylos from the project root with:"
echo "  cargo run -- --config config.toml"
echo "Then open: http://127.0.0.1:8080/app/"
