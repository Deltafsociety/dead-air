#!/usr/bin/env bash
set -e

REPO="deltafsociety/dead-air"
INSTALL_DIR="/usr/local/bin"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)
        TARGET="linux-x86_64"
        ;;
      *)
        echo "Unsupported Linux architecture: $ARCH"
        exit 1
        ;;
    esac
    ;;
  Darwin)
    # Check if running under Rosetta translation on Apple Silicon
    if [ "$ARCH" = "x86_64" ]; then
      IS_TRANSLATED="$(sysctl -in sysctl.proc_translated 2>/dev/null || echo 0)"
      if [ "$IS_TRANSLATED" = "1" ]; then
        echo "Detected Apple Silicon Mac running in x86_64 translation mode (Rosetta)."
        echo "Switching to native ARM64 release binary to prevent instruction translation issues."
        ARCH="arm64"
      fi
    fi

    case "$ARCH" in
      x86_64)
        TARGET="macos-x86_64"
        ;;
      arm64|aarch64)
        TARGET="macos-arm64"
        ;;
      *)
        echo "Unsupported macOS architecture: $ARCH"
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Unsupported operating system: $OS"
    exit 1
    ;;
esac

ARCHIVE="deadair-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/latest/download/${ARCHIVE}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading ${ARCHIVE}..."
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$URL" -o "${TMP_DIR}/${ARCHIVE}"
elif command -v wget >/dev/null 2>&1; then
  wget -q "$URL" -O "${TMP_DIR}/${ARCHIVE}"
else
  echo "Error: curl or wget is required."
  exit 1
fi

echo "Extracting binary..."
tar -xzf "${TMP_DIR}/${ARCHIVE}" -C "${TMP_DIR}"

if [ ! -f "${TMP_DIR}/deadair" ]; then
  echo "Error: deadair binary not found in archive."
  exit 1
fi

chmod +x "${TMP_DIR}/deadair"

echo "Verifying binary compatibility with host CPU..."
set +e
TEST_OUTPUT="$("${TMP_DIR}/deadair" --version 2>&1)"
TEST_STATUS=$?
set -e

if [ $TEST_STATUS -ne 0 ]; then
  echo ""
  echo "Notice: Pre-compiled binary is incompatible with host CPU instructions (${TEST_OUTPUT})."
  echo "Checking for Rust toolchain to compile natively..."

  if command -v cargo >/dev/null 2>&1; then
    echo "Rust/cargo detected. Building DEADAIR natively for your CPU..."
    CARGO_TARGET_DIR="${TMP_DIR}/cargo_build" cargo install --git "https://github.com/${REPO}.git" --root "${TMP_DIR}/cargo_out"
    if [ -f "${TMP_DIR}/cargo_out/bin/deadair" ]; then
      mv "${TMP_DIR}/cargo_out/bin/deadair" "${TMP_DIR}/deadair"
      chmod +x "${TMP_DIR}/deadair"
      echo "Native compilation completed successfully."
    else
      echo "Error: Native compilation failed."
      exit 1
    fi
  else
    echo ""
    echo "Error: The pre-compiled binary encountered an illegal hardware instruction."
    echo "This occurs when your CPU lacks instruction extensions (e.g. AVX2/BMI2) assumed by cloud runners."
    echo ""
    echo "To run DEADAIR on this machine, please compile it natively with Rust:"
    echo "  1. Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "  2. Install DEADAIR: cargo install --git https://github.com/${REPO}.git"
    exit 1
  fi
fi

if [ -w "$INSTALL_DIR" ]; then
  mv "${TMP_DIR}/deadair" "${INSTALL_DIR}/deadair"
else
  echo "Moving binary to ${INSTALL_DIR} (requires sudo)..."
  sudo mv "${TMP_DIR}/deadair" "${INSTALL_DIR}/deadair"
fi

echo "Installation complete."
echo "Launch the station anytime by running: deadair"
