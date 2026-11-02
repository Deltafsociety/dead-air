#!/usr/bin/env bash
set -euo pipefail

# DEADAIR // Android APK Build Script
echo "=========================================="
echo "  DEADAIR Mobile (Tauri v2 Android Build) "
echo "=========================================="

if [ -z "${ANDROID_HOME:-}" ]; then
  if [ -d "$HOME/Android/Sdk" ]; then
    export ANDROID_HOME="$HOME/Android/Sdk"
  elif [ -d "/usr/local/lib/android/sdk" ]; then
    export ANDROID_HOME="/usr/local/lib/android/sdk"
  else
    echo "[-] Error: ANDROID_HOME is not set and Android SDK was not found in common locations."
    echo "    Please install Android Studio / Command Line Tools and set ANDROID_HOME."
    exit 1
  fi
fi

echo "[+] Using Android SDK: $ANDROID_HOME"

# Check NDK
if [ -z "${NDK_HOME:-}" ]; then
  NDK_DIR=$(find "$ANDROID_HOME/ndk" -maxdepth 1 -mindepth 1 2>/dev/null | sort -V | tail -n 1 || true)
  if [ -n "$NDK_DIR" ]; then
    export NDK_HOME="$NDK_DIR"
    echo "[+] Detected Android NDK: $NDK_HOME"
  fi
fi

# Ensure Rust target installed
echo "[+] Ensuring Rust Android targets are installed..."
rustup target add aarch64-linux-android || true

cd "$(dirname "$0")/../gui"

# Initialize Android structure if not present
if [ ! -d "src-tauri/gen/android" ]; then
  echo "[+] Initializing Tauri Android project..."
  pnpm dlx @tauri-apps/cli android init --ci
fi

# Build release APK
echo "[+] Compiling release Android APK..."
pnpm dlx @tauri-apps/cli android build --apk --target aarch64-linux-android

APK_PATH=$(find src-tauri/gen/android -name "*.apk" 2>/dev/null | grep -E "(release|universal)" | head -n 1 || true)
if [ -n "$APK_PATH" ]; then
  mkdir -p ../dist
  cp "$APK_PATH" ../dist/deadair-android.apk
  echo "=========================================="
  echo "[+] SUCCESS: APK built successfully at:"
  echo "    dist/deadair-android.apk"
  echo "=========================================="
else
  echo "[+] Build complete. Look in gui/src-tauri/gen/android for APK outputs."
fi
