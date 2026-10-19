#!/usr/bin/env bash
# Sets up a Linux machine (x86_64) to run the factory pipeline. No sudo needed; anything that
# needs root is printed at the end. Safe to re-run: it is also the update command.
set -euo pipefail

KIT="$(cd "$(dirname "$0")" && pwd)"
NODE_MAJOR=22
RTK_VERSION=1.0.1
BIN="$HOME/.local/bin"
NODE_DIR="$HOME/.local/node"
export PATH="$NODE_DIR/bin:$BIN:$PATH"
TODO=()

step() { printf '\n== %s\n' "$*"; }
have() { command -v "$1" >/dev/null 2>&1; }

step "System tools"
for t in git curl python3 tar tmux systemctl; do
  if have "$t"; then echo "ok  $t"; else echo "MISSING $t"; TODO+=("sudo apt-get install -y $t"); fi
done
[ "$(uname -m)" = x86_64 ] || { echo "Only x86_64 Linux is supported by this script."; exit 1; }

step "Node.js $NODE_MAJOR (user space, $NODE_DIR)"
if have node && [ "$(node -p 'process.versions.node.split(".")[0]')" -ge "$NODE_MAJOR" ]; then
  echo "ok  node $(node --version)"
else
  v=$(curl -fsSL https://nodejs.org/dist/index.json | python3 -c "import json,sys; print(next(r['version'] for r in json.load(sys.stdin) if r['version'].startswith('v$NODE_MAJOR.') and r['lts']))")
  tmp=$(mktemp -d); f="node-$v-linux-x64.tar.xz"
  curl -fsSL -o "$tmp/$f" "https://nodejs.org/dist/$v/$f"
  (cd "$tmp" && curl -fsSL "https://nodejs.org/dist/$v/SHASUMS256.txt" | grep " $f\$" | sha256sum -c -)
  mkdir -p "$NODE_DIR" && tar xJf "$tmp/$f" -C "$NODE_DIR" --strip-components=1 && rm -rf "$tmp"
  echo "installed node $(node --version)"
fi
line='export PATH="$HOME/.local/node/bin:$HOME/.local/bin:$PATH"'
for rc in "$HOME/.profile" "$HOME/.bashrc"; do
  grep -qF '.local/node/bin' "$rc" 2>/dev/null || echo "$line" >> "$rc"
done

step "Claude Code"
if have claude; then echo "ok  claude $(claude --version | head -1)"
else curl -fsSL https://claude.ai/install.sh | bash; fi

step "rtk-oss $RTK_VERSION (token saver)"
if have rtk && rtk --version | grep -q "$RTK_VERSION"; then echo "ok  $(rtk --version)"
else
  tmp=$(mktemp -d); f="rtk-oss-x86_64-unknown-linux-musl.tar.gz"
  base="https://github.com/rtk-ai/rtk/releases/download/v$RTK_VERSION"
  curl -fsSL -o "$tmp/$f" "$base/$f" && curl -fsSL -o "$tmp/checksums.txt" "$base/checksums.txt"
  (cd "$tmp" && grep " $f\$" checksums.txt | sha256sum -c -)
  tar xzf "$tmp/$f" -C "$tmp" && mkdir -p "$BIN" && install -m755 "$(find "$tmp" -type f -name rtk | head -1)" "$BIN/rtk"
  rm -rf "$tmp"; echo "installed $(rtk --version)"
fi
if grep -q 'rtk hook claude' "$HOME/.claude/settings.json" 2>/dev/null; then echo "ok  rtk hook"
else rtk init --global --auto-patch; fi

step "Plugins (plugins.txt)"
known=$(claude plugin marketplace list 2>/dev/null || true)
installed=$(claude plugin list 2>/dev/null || true)
grep -vE '^\s*(#|$)' "$KIT/plugins.txt" | while read -r plugin source; do
  if [ "$source" != builtin ] && ! grep -q "($source)" <<<"$known"; then
    claude plugin marketplace add "$source" </dev/null
  fi
  if grep -q "❯ $plugin\$" <<<"$installed"; then echo "ok  $plugin"
  else claude plugin install "$plugin" </dev/null; fi
done
claude plugin marketplace update >/dev/null 2>&1 || true

step "Playwright Chromium (for ux-tester, reviewer-ui, designer)"
pw=$(npm view @playwright/mcp@latest dependencies.playwright)
npx -y "playwright@$pw" install chromium >/dev/null 2>&1 && echo "ok  chromium for playwright $pw"
chrome=$(ls -d "$HOME"/.cache/ms-playwright/chromium-*/chrome-linux*/chrome 2>/dev/null | tail -1)
if [ -n "$chrome" ] && ldd "$chrome" | grep -q 'not found'; then
  TODO+=("sudo npx -y playwright@$pw install-deps chromium   # missing: $(ldd "$chrome" | awk '/not found/{print $1}' | tr '\n' ' ')")
fi

step "Link agents + /factory skill into ~/.claude"
"$KIT/scripts/link.sh"

step "Remote Control service template"
mkdir -p "$HOME/.config/systemd/user" "$HOME/.config/claude-rc" "$HOME/projects"
chmod 700 "$HOME/.config/claude-rc"
ln -sfn "$KIT/deploy/claude-rc@.service" "$HOME/.config/systemd/user/claude-rc@.service"
systemctl --user daemon-reload && echo "ok  claude-rc@.service"
[ "$(loginctl show-user "$USER" -p Linger --value 2>/dev/null)" = yes ] || TODO+=("sudo loginctl enable-linger $USER   # keep services running while logged out")

step "Done"
claude auth status 2>/dev/null | grep -q '"loggedIn": true' || TODO+=("claude   then /login, then /exit   # sign in to your Claude plan")
if [ ${#TODO[@]} -eq 0 ]; then echo "Nothing left to do. Create a project with ./new-project.sh <name>"
else echo "Run these yourself, then re-run ./bootstrap.sh:"; printf '  %s\n' "${TODO[@]}"; fi
