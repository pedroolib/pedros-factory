#!/usr/bin/env bash
# Installs factory-dash (the live TUI dashboard) into ~/.local/bin from this repo's GitHub releases.
# Pinned version, checksum-verified. Linux (x86_64, aarch64), macOS (x86_64, arm64), WSL.
set -euo pipefail
DASH_VERSION="${DASH_VERSION:-0.1.0}"
BIN="${DASH_BIN_DIR:-$HOME/.local/bin}"

if command -v factory-dash >/dev/null 2>&1 && factory-dash --version 2>/dev/null | grep -qx "factory-dash $DASH_VERSION"; then
  echo "factory-dash $DASH_VERSION already installed: $(command -v factory-dash)"
else
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)               target=x86_64-unknown-linux-musl ;;
    Linux-aarch64|Linux-arm64)  target=aarch64-unknown-linux-musl ;;
    Darwin-arm64)               target=aarch64-apple-darwin ;;
    Darwin-x86_64)              target=x86_64-apple-darwin ;;
    *) echo "Unsupported platform $(uname -s)-$(uname -m). On Windows, use WSL." >&2; exit 1 ;;
  esac
  f="factory-dash-$target.tar.gz"
  base="https://github.com/pedroolib/pedros-factory/releases/download/dash-v$DASH_VERSION"
  tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
  curl -fsSL -o "$tmp/$f" "$base/$f"
  curl -fsSL -o "$tmp/checksums.txt" "$base/checksums.txt"
  want=$(awk -v f="$f" '$2==f || $2=="*"f {print $1}' "$tmp/checksums.txt")
  if command -v sha256sum >/dev/null; then got=$(sha256sum "$tmp/$f" | awk '{print $1}')
  else got=$(shasum -a 256 "$tmp/$f" | awk '{print $1}'); fi
  [ -n "$want" ] && [ "$want" = "$got" ] || { echo "Checksum mismatch for $f" >&2; exit 1; }
  tar xzf "$tmp/$f" -C "$tmp"
  mkdir -p "$BIN"
  install -m755 "$tmp/factory-dash" "$BIN/factory-dash"
  echo "installed $("$BIN/factory-dash" --version) to $BIN/factory-dash"
fi

case ":$PATH:" in *":$BIN:"*) ;; *) echo "NOTE: add $BIN to your PATH (e.g. in ~/.profile or ~/.zshrc)." ;; esac
echo "Run it in any terminal: factory-dash"
