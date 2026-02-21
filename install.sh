#!/usr/bin/env bash
# gittui installer
# Usage: curl -fsSL https://raw.githubusercontent.com/maplefukku/gittui/main/install.sh | bash

set -euo pipefail

REPO="maplefukku/gittui"
INSTALL_DIR="${GITTUI_INSTALL_DIR:-/usr/local/bin}"

get_latest_version() {
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" |
    grep '"tag_name"' |
    sed -E 's/.*"v([^"]+)".*/\1/'
}

get_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}" in
    Linux)
      case "${arch}" in
        x86_64)  echo "x86_64-unknown-linux-gnu" ;;
        aarch64) echo "aarch64-unknown-linux-gnu" ;;
        *)       echo "Unsupported architecture: ${arch}" >&2; exit 1 ;;
      esac
      ;;
    Darwin)
      case "${arch}" in
        x86_64)  echo "x86_64-apple-darwin" ;;
        arm64)   echo "aarch64-apple-darwin" ;;
        *)       echo "Unsupported architecture: ${arch}" >&2; exit 1 ;;
      esac
      ;;
    *)
      echo "Unsupported OS: ${os}" >&2
      echo "Please install from source: cargo install gittui" >&2
      exit 1
      ;;
  esac
}

main() {
  echo "Installing gittui..."

  local version target url tmpdir
  version="$(get_latest_version)"
  target="$(get_target)"
  url="https://github.com/${REPO}/releases/download/v${version}/gittui-${target}.tar.gz"

  echo "  Version: ${version}"
  echo "  Target:  ${target}"
  echo "  Install: ${INSTALL_DIR}/gittui"

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "${tmpdir}"' EXIT

  echo "Downloading..."
  curl -fsSL "${url}" | tar xz -C "${tmpdir}"

  if [ -w "${INSTALL_DIR}" ]; then
    mv "${tmpdir}/gittui" "${INSTALL_DIR}/gittui"
  else
    echo "Need sudo to install to ${INSTALL_DIR}"
    sudo mv "${tmpdir}/gittui" "${INSTALL_DIR}/gittui"
  fi

  chmod +x "${INSTALL_DIR}/gittui"

  echo ""
  echo "gittui v${version} installed to ${INSTALL_DIR}/gittui"
  echo "Run 'gittui' in any git repository to start!"
}

main
