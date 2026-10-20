#!/usr/bin/env bash
# Pedro's Factory server add-on: prepares a headless Linux machine (x86_64 or aarch64, systemd)
# to run factory sessions 24/7, reachable from the Claude app. No sudo needed; anything that
# needs root is printed at the end. Safe to re-run.
set -euo pipefail

KIT="$(cd "$(dirname "$0")/.." && pwd)"
NODE_MAJOR=22
BIN="$HOME/.local/bin"
NODE_DIR="$HOME/.local/node"
export PATH="$NODE_DIR/bin:$BIN:$PATH"
TODO=()

step() { printf '\n== %s\n' "$*"; }
have() { command -v "$1" >/dev/null 2>&1; }

step "System tools"
for t in git curl python3 tar tmux systemctl; do
  if have "$t"; then echo "ok  $t"; else echo "MISSING $t"; TODO+=("sudo apt-get install -y $t   # or your distro's package manager"); fi
done
case "$(uname -m)" in x86_64) narch=x64 ;; aarch64|arm64) narch=arm64 ;; *) echo "Unsupported CPU $(uname -m)"; exit 1 ;; esac

step "Node.js $NODE_MAJOR (user space, $NODE_DIR)"
if have node && [ "$(node -p 'process.versions.node.split(".")[0]')" -ge "$NODE_MAJOR" ]; then
  echo "ok  node $(node --version)"
else
  v=$(curl -fsSL https://nodejs.org/dist/index.json | python3 -c "import json,sys; print(next(r['version'] for r in json.load(sys.stdin) if r['version'].startswith('v$NODE_MAJOR.') and r['lts']))")
  tmp=$(mktemp -d); f="node-$v-linux-$narch.tar.xz"
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

step "rtk (token saver)"
bash "$KIT/plugins/factory/scripts/install-rtk.sh" | grep -v '^Next:'
if grep -q 'rtk hook claude' "$HOME/.claude/settings.json" 2>/dev/null; then echo "ok  rtk hook"
else rtk init --global --auto-patch; fi

step "Pedro's Factory plugins"
if claude plugin marketplace list 2>/dev/null | grep -q 'pedros-factory'; then
  claude plugin marketplace update pedros-factory >/dev/null 2>&1 || true; echo "ok  marketplace"
else claude plugin marketplace add pedroolib/pedros-factory </dev/null; fi
if claude plugin list 2>/dev/null | grep -q '❯ factory@pedros-factory$'; then echo "ok  factory"
else claude plugin install factory@pedros-factory </dev/null; fi

step "Remote Control service template"
mkdir -p "$HOME/.config/systemd/user" "$HOME/.config/claude-rc" "$HOME/projects"
chmod 700 "$HOME/.config/claude-rc"
ln -sfn "$KIT/server/claude-rc@.service" "$HOME/.config/systemd/user/claude-rc@.service"
systemctl --user daemon-reload && echo "ok  claude-rc@.service"
[ "$(loginctl show-user "$USER" -p Linger --value 2>/dev/null)" = yes ] || TODO+=("sudo loginctl enable-linger $USER   # keep services running while logged out")

step "Done"
git config --global user.email >/dev/null || TODO+=("git config --global user.name 'Your Name' && git config --global user.email you@example.com")
claude auth status 2>/dev/null | grep -q '"loggedIn": true' || TODO+=("claude   then /login, then /exit")
if [ ${#TODO[@]} -eq 0 ]; then
  echo "Server ready. For each project: put it in ~/projects/<name>, run 'claude' there once to"
  echo "trust the folder and run /factory:init, then 'claude remote-control' once (accept, choose"
  echo "2 = worktree, Ctrl+C), then: systemctl --user enable --now claude-rc@<name>"
else
  echo "Run these yourself, then re-run server/bootstrap.sh:"; printf '  %s\n' "${TODO[@]}"
fi
