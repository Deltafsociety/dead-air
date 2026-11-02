#!/usr/bin/env bash
# ==============================================================================
# DEADAIR // Automated Source Installer (Linux & macOS)
# Sovereign offline-first Nostr station for blackouts and intranet survival
# ==============================================================================

set -euo pipefail

REPO="deltafsociety/dead-air"
GIT_URL="https://github.com/${REPO}.git"
TARBALL_URL="https://github.com/${REPO}/archive/refs/heads/master.tar.gz"

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BOLD='\033[1m'
NC='\033[0m' # No Color

echo -e "${CYAN}${BOLD}"
cat << 'EOF'
   ____  _________    ____  ___    ________ 
  / __ \/ ____/   |  / __ \/   |  /  _/ __ \
 / / / / __/ / /| | / / / / /| |  / // /_/ /
/ /_/ / /___/ ___ |/ /_/ / ___ |_/ // _, _/ 
/_____/_____/_/  |_/_____/_/  |_/___/_/ |_|  
EOF
echo -e "${NC}"
echo -e "${GREEN}${BOLD}>> DEADAIR Native Source Installer${NC}"
echo -e "   Target: Building native binary from source with custom embedded icon"
echo ""

# ------------------------------------------------------------------------------
# 1. Edition Selection (Interactive or via $DEADAIR_TARGET)
# ------------------------------------------------------------------------------
echo -e "${CYAN}Please select the DEADAIR edition you want to install:${NC}"
echo -e "  ${BOLD}[1]${NC} Terminal TUI Station (command: ${CYAN}deadair${NC}) ${GREEN}[DEFAULT]${NC}"
echo -e "  ${BOLD}[2]${NC} Desktop GUI Station (adds to your Application Menu with custom icon)"
echo -e "  ${BOLD}[3]${NC} Both (Terminal TUI + Desktop GUI)"
echo ""

TARGET_CHOICE="${DEADAIR_TARGET:-}"
if [ -z "$TARGET_CHOICE" ]; then
  if [ -t 0 ]; then
    read -r -p "Enter choice [1/2/3] (default: 1): " TARGET_CHOICE || true
  elif [ -e /dev/tty ]; then
    read -r -p "Enter choice [1/2/3] (default: 1): " TARGET_CHOICE </dev/tty || true
  fi
fi

TARGET_CHOICE="${TARGET_CHOICE:-1}"
BUILD_TUI=0
BUILD_GUI=0

case "$TARGET_CHOICE" in
  2|gui|GUI)
    BUILD_GUI=1
    echo -e "[+] Selected: ${BOLD}Desktop GUI Station${NC}"
    ;;
  3|both|all|ALL)
    BUILD_TUI=1
    BUILD_GUI=1
    echo -e "[+] Selected: ${BOLD}Both Terminal TUI and Desktop GUI${NC}"
    ;;
  *)
    BUILD_TUI=1
    echo -e "[+] Selected: ${BOLD}Terminal TUI Station${NC}"
    ;;
esac

echo ""

# ------------------------------------------------------------------------------
# 2. System & Architecture Detection
# ------------------------------------------------------------------------------
OS="$(uname -s)"
ARCH="$(uname -m)"

echo -e "[+] Detected Platform: ${BOLD}${OS} (${ARCH})${NC}"

# Check if elevated privileges or non-interactive sudo are available
can_sudo() {
  command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null
}

run_elevated() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  elif can_sudo; then
    sudo "$@"
  else
    return 1
  fi
}

# ------------------------------------------------------------------------------
# 3. Essential Package Dependencies
# ------------------------------------------------------------------------------
ensure_package() {
  local PKG_NAME="$1"
  local CMD_CHECK="$2"

  if command -v "$CMD_CHECK" >/dev/null 2>&1; then
    return 0
  fi

  echo -e "[+] Missing prerequisite: ${YELLOW}${PKG_NAME}${NC}. Attempting installation..."

  if command -v apt-get >/dev/null 2>&1 && ( [ "$(id -u)" -eq 0 ] || can_sudo ); then
    run_elevated apt-get update -qq
    run_elevated apt-get install -y -qq "$PKG_NAME"
  elif command -v dnf >/dev/null 2>&1 && ( [ "$(id -u)" -eq 0 ] || can_sudo ); then
    run_elevated dnf install -y -q "$PKG_NAME"
  elif command -v pacman >/dev/null 2>&1 && ( [ "$(id -u)" -eq 0 ] || can_sudo ); then
    run_elevated pacman -Sy --noconfirm "$PKG_NAME"
  elif command -v zypper >/dev/null 2>&1 && ( [ "$(id -u)" -eq 0 ] || can_sudo ); then
    run_elevated zypper --non-interactive install -q "$PKG_NAME"
  elif command -v apk >/dev/null 2>&1 && ( [ "$(id -u)" -eq 0 ] || can_sudo ); then
    run_elevated apk add --no-cache "$PKG_NAME"
  elif command -v brew >/dev/null 2>&1; then
    brew install "$PKG_NAME"
  else
    echo -e "${YELLOW}[!] Warning: Could not install ${PKG_NAME} automatically without root/sudo.${NC}"
    return 0
  fi
}

if ! command -v curl >/dev/null 2>&1; then
  ensure_package curl curl
fi

if ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1 && ! command -v clang >/dev/null 2>&1; then
  echo -e "[+] C compiler not found. Installing build tools..."
  if command -v apt-get >/dev/null 2>&1 && ( [ "$(id -u)" -eq 0 ] || can_sudo ); then
    run_elevated apt-get update -qq && run_elevated apt-get install -y -qq build-essential
  elif command -v dnf >/dev/null 2>&1 && ( [ "$(id -u)" -eq 0 ] || can_sudo ); then
    run_elevated dnf groupinstall -y -q "Development Tools" || run_elevated dnf install -y -q gcc gcc-c++ make
  elif command -v pacman >/dev/null 2>&1 && ( [ "$(id -u)" -eq 0 ] || can_sudo ); then
    run_elevated pacman -Sy --noconfirm base-devel
  elif command -v apk >/dev/null 2>&1 && ( [ "$(id -u)" -eq 0 ] || can_sudo ); then
    run_elevated apk add --no-cache build-base
  elif [ "$OS" = "Darwin" ]; then
    xcode-select --install || true
  fi
fi

if ! command -v git >/dev/null 2>&1; then
  ensure_package git git
fi

# GUI-specific build libraries on Linux
if [ "$BUILD_GUI" -eq 1 ] && [ "$OS" = "Linux" ]; then
  if ! (pkg-config --exists gtk+-3.0 webkit2gtk-4.1 2>/dev/null || pkg-config --exists gtk+-3.0 webkit2gtk-4.0 2>/dev/null); then
    echo -e "[+] Desktop GUI requires WebKit2GTK and GTK3 development headers..."
    if command -v apt-get >/dev/null 2>&1 && ( [ "$(id -u)" -eq 0 ] || can_sudo ); then
      run_elevated apt-get update -qq
      run_elevated apt-get install -y -qq libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev || \
      run_elevated apt-get install -y -qq libwebkit2gtk-4.0-dev libgtk-3-dev libappindicator3-dev librsvg2-dev
    elif command -v dnf >/dev/null 2>&1 && ( [ "$(id -u)" -eq 0 ] || can_sudo ); then
      run_elevated dnf install -y -q webkit2gtk4.1-devel gtk3-devel libayatana-appindicator-devel librsvg2-devel || \
      run_elevated dnf install -y -q webkit2gtk3-devel gtk3-devel
    elif command -v pacman >/dev/null 2>&1 && ( [ "$(id -u)" -eq 0 ] || can_sudo ); then
      run_elevated pacman -Sy --noconfirm webkit2gtk-4.1 gtk3 libayatana-appindicator librsvg || \
      run_elevated pacman -Sy --noconfirm webkit2gtk gtk3
    elif command -v zypper >/dev/null 2>&1 && ( [ "$(id -u)" -eq 0 ] || can_sudo ); then
      run_elevated zypper --non-interactive install -q webkit2gtk3-devel gtk3-devel libayatana-appindicator3-devel librsvg-devel
    fi
  fi
fi

# ------------------------------------------------------------------------------
# 4. Rust Toolchain (Cargo & Rustc)
# ------------------------------------------------------------------------------
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck source=/dev/null
  . "$HOME/.cargo/env"
elif [ -d "/usr/local/cargo/bin" ]; then
  export PATH="/usr/local/cargo/bin:$PATH"
fi

if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
  echo -e "${YELLOW}[!] Rust toolchain not detected.${NC}"
  echo -e "[+] Installing Rust stable (minimal profile via rustup)..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
  # shellcheck source=/dev/null
  . "$HOME/.cargo/env"
  echo -e "${GREEN}[+] Rust toolchain successfully installed!${NC}"
fi

echo -e "[+] Using Rust toolchain: ${BOLD}$(rustc --version)${NC}"

# ------------------------------------------------------------------------------
# 5. Source Tree Retrieval
# ------------------------------------------------------------------------------
TMP_DIR="$(mktemp -d)"
CLEANUP_REQUIRED=1

cleanup() {
  if [ "$CLEANUP_REQUIRED" -eq 1 ] && [ -d "$TMP_DIR" ]; then
    rm -rf "$TMP_DIR"
  fi
}
trap cleanup EXIT

SOURCE_DIR=""
if [ -f "Cargo.toml" ] && grep -q 'name = "deadair"' "Cargo.toml" 2>/dev/null; then
  echo -e "[+] Found local DEADAIR source tree in current working directory."
  SOURCE_DIR="$(pwd)"
elif [ -f "../Cargo.toml" ] && grep -q 'name = "deadair"' "../Cargo.toml" 2>/dev/null; then
  echo -e "[+] Found local DEADAIR source tree in parent directory."
  SOURCE_DIR="$(cd .. && pwd)"
else
  echo -e "[+] Fetching latest DEADAIR source from ${BOLD}${REPO}${NC}..."
  SOURCE_DIR="${TMP_DIR}/source"
  if command -v git >/dev/null 2>&1; then
    git clone --depth 1 "$GIT_URL" "$SOURCE_DIR"
  else
    mkdir -p "$SOURCE_DIR"
    curl -fsSL "$TARBALL_URL" | tar -xz --strip-components=1 -C "$SOURCE_DIR"
  fi
fi

cd "$SOURCE_DIR"

# ------------------------------------------------------------------------------
# 6. Build from Source
# ------------------------------------------------------------------------------
if [ "$BUILD_TUI" -eq 1 ]; then
  echo -e "[+] Compiling Terminal TUI Station from source (release profile)..."
  cargo build --release -p deadair
fi

if [ "$BUILD_GUI" -eq 1 ]; then
  echo -e "[+] Compiling Desktop GUI Station from source (release profile)..."
  cargo build --release -p deadair-gui
fi

echo -e "${GREEN}[+] Compilation completed successfully!${NC}"

# ------------------------------------------------------------------------------
# 7. Installation & Desktop Menu Registration
# ------------------------------------------------------------------------------
INSTALL_DIR="/usr/local/bin"
choose_install_dir() {
  if [ -w "$INSTALL_DIR" ] || can_sudo; then
    echo "/usr/local/bin"
  elif [ -d "$HOME/.cargo/bin" ]; then
    echo "$HOME/.cargo/bin"
  else
    echo "$HOME/.local/bin"
  fi
}

TARGET_INSTALL_DIR="$(choose_install_dir)"

copy_bin() {
  local SRC="$1"
  local DEST="$2"

  if [ -w "$(dirname "$DEST")" ]; then
    cp "$SRC" "$DEST"
    chmod 755 "$DEST"
  elif can_sudo; then
    sudo cp "$SRC" "$DEST"
    sudo chmod 755 "$DEST"
  else
    mkdir -p "$(dirname "$DEST")"
    cp "$SRC" "$DEST"
    chmod 755 "$DEST"
  fi
}

# Install TUI binary
if [ "$BUILD_TUI" -eq 1 ]; then
  TUI_COMPILED="${SOURCE_DIR}/target/release/deadair"
  TUI_DEST="${TARGET_INSTALL_DIR}/deadair"
  copy_bin "$TUI_COMPILED" "$TUI_DEST"
  echo -e "[+] Installed Terminal TUI: ${BOLD}${TUI_DEST}${NC}"
fi

# Install GUI binary & Desktop Entry
if [ "$BUILD_GUI" -eq 1 ]; then
  GUI_COMPILED="${SOURCE_DIR}/target/release/deadair-gui"
  GUI_DEST="${TARGET_INSTALL_DIR}/deadair-gui"
  copy_bin "$GUI_COMPILED" "$GUI_DEST"
  echo -e "[+] Installed Desktop GUI: ${BOLD}${GUI_DEST}${NC}"

  # Register to Application Menu on Linux
  if [ "$OS" = "Linux" ]; then
    APPS_DIR="$HOME/.local/share/applications"
    ICONS_DIR="$HOME/.local/share/icons/hicolor/512x512/apps"
    PIXMAPS_DIR="$HOME/.local/share/pixmaps"

    mkdir -p "$APPS_DIR" "$ICONS_DIR" "$PIXMAPS_DIR"

    if [ -f "${SOURCE_DIR}/assets/icon.png" ]; then
      cp "${SOURCE_DIR}/assets/icon.png" "${ICONS_DIR}/deadair.png"
      cp "${SOURCE_DIR}/assets/icon.png" "${PIXMAPS_DIR}/deadair.png"
    fi

    DESKTOP_ENTRY="${APPS_DIR}/deadair.desktop"
    cat > "$DESKTOP_ENTRY" << EOF
[Desktop Entry]
Name=DEADAIR
GenericName=Intranet Nostr Station
Comment=Offline-first sovereign Nostr station for blackouts and intranet mesh survival
Exec=${GUI_DEST}
Icon=deadair
Terminal=false
Type=Application
Categories=Network;Chat;InstantMessaging;
Keywords=nostr;mesh;intranet;blackout;offline;
StartupWMClass=deadair
EOF
    chmod 644 "$DESKTOP_ENTRY"

    if command -v update-desktop-database >/dev/null 2>&1; then
      update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
      gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" >/dev/null 2>&1 || true
    fi

    echo -e "${GREEN}[+] Added DEADAIR to Application Menu with custom cybernetic icon!${NC}"
  fi
fi

# Check PATH
case ":${PATH}:" in
  *":${TARGET_INSTALL_DIR}:"*) ;;
  *)
    echo -e "${YELLOW}[!] Notice: ${TARGET_INSTALL_DIR} is not in your current PATH.${NC}"
    echo -e "    Add it to your shell by running: export PATH=\"${TARGET_INSTALL_DIR}:\$PATH\""
    echo -e "    Or add that line to ~/.bashrc or ~/.zshrc."
    ;;
esac

# ------------------------------------------------------------------------------
# 8. Success Summary
# ------------------------------------------------------------------------------
echo ""
echo -e "${GREEN}${BOLD}================================================================${NC}"
echo -e "${GREEN}${BOLD}  DEADAIR has been successfully built and installed from source! ${NC}"
echo -e "${GREEN}${BOLD}================================================================${NC}"
echo ""
if [ "$BUILD_TUI" -eq 1 ]; then
  echo -e "  * Terminal TUI: Run anytime in terminal with command: ${CYAN}${BOLD}deadair${NC}"
fi
if [ "$BUILD_GUI" -eq 1 ]; then
  echo -e "  * Desktop GUI:  Launch from your Application Menu (search '${CYAN}${BOLD}DEADAIR${NC}')"
  echo -e "                  Or run via CLI: ${CYAN}${BOLD}deadair-gui${NC}"
fi
echo ""
echo -e "Enjoy sovereign and blackout-proof intranet communications."
