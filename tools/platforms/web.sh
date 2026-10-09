#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
rustup target add wasm32-unknown-unknown
tools/cargo build -p incant_platform_smoke --lib --release --target wasm32-unknown-unknown --locked
mkdir -p artifacts/web
wasm-bindgen --target web --out-dir artifacts/web/pkg target/wasm32-unknown-unknown/release/incant_platform_smoke.wasm
cp platforms/web/index.html artifacts/web/index.html
