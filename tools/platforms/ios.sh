#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
mode="${1:-simulator}"
if [[ "$mode" == simulator ]]; then
  target=aarch64-apple-ios-sim
  sdk=iphonesimulator
  swift_target=arm64-apple-ios15.0-simulator
elif [[ "$mode" == device ]]; then
  target=aarch64-apple-ios
  sdk=iphoneos
  swift_target=arm64-apple-ios15.0
else
  echo 'Usage: ios.sh simulator|device' >&2
  exit 1
fi
rustup target add "$target"
tools/cargo build -p incant_platform_smoke --release --lib --target "$target" --locked
out="artifacts/ios-$mode/IncantSmoke.app"
mkdir -p "$out"
xcrun --sdk "$sdk" swiftc -target "$swift_target" -sdk "$(xcrun --sdk "$sdk" --show-sdk-path)" -parse-as-library platforms/ios/main.swift "target/$target/release/libincant_platform_smoke.a" -framework UIKit -framework Foundation -framework Security -lresolv -lc++ -o "$out/IncantSmoke"
cp platforms/ios/Info.plist "$out/Info.plist"
if [[ "$mode" == simulator ]]; then
  codesign --force --sign - "$out"
else
  echo 'Device .app is unsigned; installation requires director-provided provisioning/signing.'
fi
