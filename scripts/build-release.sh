#!/usr/bin/env bash
# Build the single NicheRS executable with the web UI embedded (twelve-factor V: build stage).
# Requirements at build time only: Node.js ≥ 20 + npm, Rust (see rust-version in Cargo.toml).
# Output: target/dist/niche (no Node/Java/Python needed at runtime).
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
BUILD_ID="$(git describe --always --dirty 2>/dev/null || echo unknown)"
echo "==> web UI ($BUILD_ID)"
(cd web && npm ci --no-audit --no-fund && VITE_BUILD_ID="$BUILD_ID" npm run build)
echo "==> tests"
cargo test --workspace --quiet
echo "==> executable"
NICHE_BUILD_ID="$BUILD_ID" cargo build --profile dist -p niche-cli
ls -lh target/dist/niche
echo "Run: ./target/dist/niche serve   (then open http://127.0.0.1:8080)"
