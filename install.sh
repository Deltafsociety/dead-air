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

if [ -w "$INSTALL_DIR" ]; then
  mv "${TMP_DIR}/deadair" "${INSTALL_DIR}/deadair"
else
  echo "Moving binary to ${INSTALL_DIR} (requires sudo)..."
  sudo mv "${TMP_DIR}/deadair" "${INSTALL_DIR}/deadair"
fi

echo "Installation complete."
echo "Launch the station anytime by running: deadair"
