#!/bin/bash
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
if [ ! -f /opt/incant-rust/.ready ]; then
apt-get update
apt-get install -y --no-install-recommends ca-certificates curl xz-utils build-essential pkg-config libgtk-3-dev libdbus-1-dev
mkdir -p /opt/incant-rust /tmp/rust-installer
cd /tmp/rust-installer
curl --fail --location --retry 3 --proto =https --tlsv1.2 https://static.rust-lang.org/dist/2026-10-01/rustc-1.99.0-aarch64-unknown-linux-gnu.tar.xz --output rustc-1.99.0-aarch64-unknown-linux-gnu.tar.xz
printf '%s\n' '89c0f1a3a44df63c95e5d5f2ba093d0a12c5f5f57f8635f8a7a0ce50cf86600e  rustc-1.99.0-aarch64-unknown-linux-gnu.tar.xz' | sha256sum --check --strict
tar -xf rustc-1.99.0-aarch64-unknown-linux-gnu.tar.xz
./rustc-1.99.0-aarch64-unknown-linux-gnu/install.sh --prefix=/opt/incant-rust --disable-ldconfig
rm -rf rustc-1.99.0-aarch64-unknown-linux-gnu rustc-1.99.0-aarch64-unknown-linux-gnu.tar.xz
curl --fail --location --retry 3 --proto =https --tlsv1.2 https://static.rust-lang.org/dist/2026-10-01/cargo-1.99.0-aarch64-unknown-linux-gnu.tar.xz --output cargo-1.99.0-aarch64-unknown-linux-gnu.tar.xz
printf '%s\n' '113229fcc16cc1ba004923a4cc275fbf3281dafaaf8b97269f9c00189d497fd4  cargo-1.99.0-aarch64-unknown-linux-gnu.tar.xz' | sha256sum --check --strict
tar -xf cargo-1.99.0-aarch64-unknown-linux-gnu.tar.xz
./cargo-1.99.0-aarch64-unknown-linux-gnu/install.sh --prefix=/opt/incant-rust --disable-ldconfig
rm -rf cargo-1.99.0-aarch64-unknown-linux-gnu cargo-1.99.0-aarch64-unknown-linux-gnu.tar.xz
curl --fail --location --retry 3 --proto =https --tlsv1.2 https://static.rust-lang.org/dist/2026-10-01/rust-std-1.99.0-aarch64-unknown-linux-gnu.tar.xz --output rust-std-1.99.0-aarch64-unknown-linux-gnu.tar.xz
printf '%s\n' '1cf7e2ef58ed1cfa6f1adea18e155cbf254c1951f39e5f5e3376879f55fd64a4  rust-std-1.99.0-aarch64-unknown-linux-gnu.tar.xz' | sha256sum --check --strict
tar -xf rust-std-1.99.0-aarch64-unknown-linux-gnu.tar.xz
./rust-std-1.99.0-aarch64-unknown-linux-gnu/install.sh --prefix=/opt/incant-rust --disable-ldconfig
rm -rf rust-std-1.99.0-aarch64-unknown-linux-gnu rust-std-1.99.0-aarch64-unknown-linux-gnu.tar.xz
touch /opt/incant-rust/.ready
fi
export PATH=/opt/incant-rust/bin:$PATH
export CARGO_HOME=/state/cargo
export CARGO_TARGET_DIR=/state/target
cd /source
rustc -vV
pkg-config --modversion gtk+-3.0
cargo check --locked --profile ci -j2 -p tao
