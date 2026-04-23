#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'; YELLOW='\033[0;33m'; GREEN='\033[0;32m'; RESET='\033[0m'
ok()   { echo -e "${GREEN}  ✓${RESET} $*"; }
warn() { echo -e "${YELLOW}  !${RESET} $*"; }
err()  { echo -e "${RED}  ✗${RESET} $*" >&2; exit 1; }

VERSION=""
BIN_DIR="${HOME}/.local/bin"
SHELL_INTEGRATION=true

usage() {
  cat <<EOF
Install branch — an interactive git branch and worktree navigator.

USAGE:
  curl -fsSL https://raw.githubusercontent.com/piotr-lebski/branch/main/install.sh | bash
  bash install.sh [FLAGS]

FLAGS:
  --version <ver>         Install a specific release (e.g. v0.1.0). Default: latest.
  --bin-dir <path>        Directory to install the binary. Default: ~/.local/bin
  --no-shell-integration  Skip adding the shell init line to your config file.
  -h, --help              Show this message.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) [[ $# -ge 2 ]] || err "--version requires an argument"; VERSION="$2"; shift 2 ;;
    --bin-dir)  [[ $# -ge 2 ]] || err "--bin-dir requires an argument"; BIN_DIR="$2"; shift 2 ;;
    --no-shell-integration) SHELL_INTEGRATION=false; shift ;;
    -h|--help) usage; exit 0 ;;
    *) err "Unknown flag: $1. Run with --help for usage." ;;
  esac
done

command -v curl &>/dev/null || err "curl is required. Install it and re-run."

OS=$(uname -s)
ARCH=$(uname -m)
case "${OS}-${ARCH}" in
  Linux-x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
  Darwin-arm64)  TARGET="aarch64-apple-darwin" ;;
  *) err "Unsupported platform: ${OS} ${ARCH}. See https://github.com/piotr-lebski/branch#install to build from source." ;;
esac

if [[ -z "$VERSION" ]]; then
  VERSION=$(curl -fsSL "https://api.github.com/repos/piotr-lebski/branch/releases/latest" \
    | grep '"tag_name"' \
    | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
  [[ -n "$VERSION" ]] || err "Failed to determine latest release version."
fi

echo "Installing branch ${VERSION} for ${TARGET}…"

BRANCH_TMP=$(mktemp -d)
trap 'rm -rf "$BRANCH_TMP"' EXIT
ARCHIVE="branch-${TARGET}-${VERSION}.tar.gz"
BASE_URL="https://github.com/piotr-lebski/branch/releases/download/${VERSION}"

curl -fsSL "${BASE_URL}/${ARCHIVE}"        -o "${BRANCH_TMP}/${ARCHIVE}"
curl -fsSL "${BASE_URL}/${ARCHIVE}.sha256" -o "${BRANCH_TMP}/${ARCHIVE}.sha256"

cd "$BRANCH_TMP"
if command -v sha256sum &>/dev/null; then
  sha256sum --check "${ARCHIVE}.sha256" --quiet
else
  shasum -a 256 -c "${ARCHIVE}.sha256" --quiet
fi
ok "Checksum verified"

tar -xzf "${BRANCH_TMP}/${ARCHIVE}" -C "$BRANCH_TMP"
mkdir -p "$BIN_DIR"
mv -f "${BRANCH_TMP}/branch" "${BIN_DIR}/branch"
chmod +x "${BIN_DIR}/branch"
ok "Installed to ${BIN_DIR}/branch"

detect_shell() {
  case "${SHELL:-}" in
    */bash) echo "bash" ;;
    */zsh)  echo "zsh"  ;;
    */fish) echo "fish" ;;
    *)      echo "unknown" ;;
  esac
}

shell_config_file() {
  local sh="$1"
  case "$sh" in
    bash) if [[ "$OS" == "Darwin" ]]; then echo "${HOME}/.bash_profile"; else echo "${HOME}/.bashrc"; fi ;;
    zsh)  echo "${HOME}/.zshrc" ;;
    fish) echo "${HOME}/.config/fish/config.fish" ;;
    *)    echo "" ;;
  esac
}

DETECTED_SHELL=$(detect_shell)
CONFIG_FILE=$(shell_config_file "$DETECTED_SHELL")

if [[ -z "$CONFIG_FILE" ]]; then
  warn "Could not detect your shell (SHELL=${SHELL:-unset})."
  warn "Add the following to your shell config manually:"
  warn "  export PATH=\"${BIN_DIR}:\$PATH\""
  warn '  eval "$(branch --init)"'
else
  mkdir -p "$(dirname "$CONFIG_FILE")"

  if ! grep -qF "${BIN_DIR}:" "$CONFIG_FILE" 2>/dev/null && \
     ! grep -qF "\"${BIN_DIR}\"" "$CONFIG_FILE" 2>/dev/null; then
    printf '\n' >> "$CONFIG_FILE"
    if [[ "$DETECTED_SHELL" == "fish" ]]; then
      echo "fish_add_path \"${BIN_DIR}\"" >> "$CONFIG_FILE"
    else
      echo "export PATH=\"${BIN_DIR}:\$PATH\"" >> "$CONFIG_FILE"
    fi
    ok "Added ${BIN_DIR} to PATH in ${CONFIG_FILE}"
  fi

  if [[ "$SHELL_INTEGRATION" == "true" ]]; then
    if grep -q "branch --init" "$CONFIG_FILE" 2>/dev/null; then
      ok "Shell integration already present in ${CONFIG_FILE}"
    else
      if [[ "$DETECTED_SHELL" == "fish" ]]; then
        echo 'branch --init | source' >> "$CONFIG_FILE"
      else
        echo 'eval "$(branch --init)"' >> "$CONFIG_FILE"
      fi
      ok "Added shell integration to ${CONFIG_FILE}"
    fi
  else
    echo ""
    echo "  Shell init skipped. Add the following to ${CONFIG_FILE} manually:"
    echo '    eval "$(branch --init)"'
  fi
fi

echo ""
ok "branch ${VERSION} installed successfully!"
echo ""
if [[ -n "$CONFIG_FILE" ]]; then
  echo "  Restart your shell or run: source ${CONFIG_FILE}"
else
  echo "  Add ${BIN_DIR} to your PATH and the init line to your shell config, then restart your shell."
fi
