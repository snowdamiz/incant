#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to an installed Android NDK}"
rustup target add aarch64-linux-android x86_64-linux-android
cargo ndk -t arm64-v8a -t x86_64 -o platforms/android/app/src/main/jniLibs build -p incant_platform_smoke --lib --release --locked
gradle --no-daemon -p platforms/android assembleDebug
mkdir -p artifacts/android
cp platforms/android/app/build/outputs/apk/debug/app-debug.apk artifacts/android/IncantSmoke-debug.apk
