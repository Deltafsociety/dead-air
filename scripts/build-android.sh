#!/usr/bin/env bash
set -euo pipefail

# DEADAIR // Android APK Build Script (Signed & Aligned)
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

# Locate build-tools (zipalign & apksigner)
BUILD_TOOLS_DIR=$(find "$ANDROID_HOME/build-tools" -maxdepth 1 -mindepth 1 2>/dev/null | sort -V | tail -n 1 || true)
if [ -z "$BUILD_TOOLS_DIR" ]; then
  echo "[-] Warning: Android build-tools not found under $ANDROID_HOME/build-tools"
fi

# Ensure Rust Android targets installed
echo "[+] Ensuring Rust Android targets are installed..."
rustup target add aarch64-linux-android armv7-linux-androideabi || true

cd "$(dirname "$0")/../gui"

# Initialize Android structure if not present
if [ ! -d "src-tauri/gen/android" ]; then
  echo "[+] Initializing Tauri Android project..."
  pnpm dlx @tauri-apps/cli android init --ci
fi

# Ensure network permissions in AndroidManifest.xml
MANIFEST="src-tauri/gen/android/app/src/main/AndroidManifest.xml"
if [ -f "$MANIFEST" ]; then
  if ! grep -q "CHANGE_WIFI_MULTICAST_STATE" "$MANIFEST"; then
    echo "[+] Adding domestic Wi-Fi & UDP multicast permissions to AndroidManifest.xml..."
    sed -i '/<manifest/a \    <uses-permission android:name="android.permission.INTERNET" />\n    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />\n    <uses-permission android:name="android.permission.CHANGE_WIFI_MULTICAST_STATE" />\n    <uses-permission android:name="android.permission.ACCESS_WIFI_STATE" />' "$MANIFEST"
  fi
fi

# Build APKs
echo "[+] Compiling Android APKs (aarch64 + armv7)..."
pnpm dlx @tauri-apps/cli android build --apk --target aarch64 --target armv7 --ci

# Locate raw built APK
RAW_APK=$(find src-tauri/gen/android/app/build/outputs/apk -name "*.apk" 2>/dev/null | head -n 1 || true)

if [ -z "$RAW_APK" ]; then
  echo "[-] Error: No APK found after build in src-tauri/gen/android/app/build/outputs/apk"
  exit 1
fi

echo "[+] Found raw APK: $RAW_APK"
mkdir -p ../dist

# Align and Sign APK if build-tools available
if [ -n "$BUILD_TOOLS_DIR" ] && [ -x "$BUILD_TOOLS_DIR/apksigner" ]; then
  echo "[+] Generating local signing key (if not present)..."
  if [ ! -f "release.keystore" ]; then
    keytool -genkeypair -v \
      -keystore release.keystore \
      -alias deadair \
      -keyalg RSA \
      -keysize 2048 \
      -validity 10000 \
      -storepass deadairpass \
      -keypass deadairpass \
      -dname "CN=DEADAIR, OU=Mobile, O=deltafsociety, L=Tehran, C=IR"
  fi

  echo "[+] Zip-aligning APK..."
  if [ -x "$BUILD_TOOLS_DIR/zipalign" ]; then
    "$BUILD_TOOLS_DIR/zipalign" -v -p -f 4 "$RAW_APK" aligned.apk
    TARGET_FOR_SIGN="aligned.apk"
  else
    TARGET_FOR_SIGN="$RAW_APK"
  fi

  echo "[+] Signing APK with APK Signature Scheme v2/v3..."
  "$BUILD_TOOLS_DIR/apksigner" sign \
    --ks release.keystore \
    --ks-pass pass:deadairpass \
    --key-pass pass:deadairpass \
    --out ../dist/deadair-android.apk \
    "$TARGET_FOR_SIGN"

  echo "[+] Verifying APK signature..."
  "$BUILD_TOOLS_DIR/apksigner" verify --verbose ../dist/deadair-android.apk

  rm -f aligned.apk
else
  echo "[!] Warning: apksigner not found, copying raw APK directly"
  cp "$RAW_APK" ../dist/deadair-android.apk
fi

echo "=========================================="
echo "[+] SUCCESS: Valid signed APK ready at:"
echo "    dist/deadair-android.apk"
echo "=========================================="
