#!/usr/bin/env bash
# Adds the factory pipeline to ~/projects/<name> (creates the folder and git repo if needed).
# Never overwrites files that already exist.
#
#   ./new-project.sh <name> [--ui heroui|shadcn|none] [--supabase-ref <ref> | --no-db]
set -euo pipefail

usage() { sed -n '2,6p' "$0" | sed 's/^# \{0,1\}//'; exit 1; }
[ $# -ge 1 ] || usage
NAME=$1; shift
UI=heroui; REF=""; DB=yes
while [ $# -gt 0 ]; do
  case $1 in
    --ui) UI=$2; shift 2 ;;
    --supabase-ref) REF=$2; shift 2 ;;
    --no-db) DB=no; shift ;;
    *) usage ;;
  esac
done
case $UI in heroui|shadcn|none) ;; *) echo "--ui must be heroui, shadcn or none"; exit 1 ;; esac
[[ $NAME =~ ^[A-Za-z0-9._-]+$ ]] || { echo "name may only contain letters, digits, . _ -"; exit 1; }
[ "$DB" = no ] || [ -n "$REF" ] || { echo "pass --supabase-ref <ref> (from supabase.com/dashboard/project/<ref>) or --no-db"; exit 1; }

KIT="$(cd "$(dirname "$0")" && pwd)"
DIR="$HOME/projects/$NAME"
mkdir -p "$DIR" && cd "$DIR"
NEW_REPO=no
[ -d .git ] || { git init -q -b main; NEW_REPO=yes; }

write_if_absent() { # <path>  (content on stdin)
  if [ -e "$1" ]; then echo "keep    $1 (already exists)"; cat >/dev/null
  else mkdir -p "$(dirname "$1")"; cat > "$1"; echo "created $1"; fi
}

write_if_absent .claude/settings.json < "$KIT/project-template/.claude/settings.json"

UI="$UI" REF="$REF" DB="$DB" python3 - <<'EOF' | write_if_absent .mcp.json
import json, os
servers = {}
ui = os.environ["UI"]
if ui == "heroui": servers["ui-kit"] = {"command": "npx", "args": ["-y", "@heroui/react-mcp@latest"]}
if ui == "shadcn": servers["ui-kit"] = {"command": "npx", "args": ["-y", "shadcn@latest", "mcp"]}
if os.environ["DB"] == "yes":
    base = "https://mcp.supabase.com/mcp?project_ref=" + os.environ["REF"]
    auth = {"Authorization": "Bearer ${SUPABASE_ACCESS_TOKEN}"}
    servers["db"] = {"type": "http", "url": base, "headers": auth}
    servers["db-readonly"] = {"type": "http", "url": base + "&read_only=true", "headers": auth}
print(json.dumps({"mcpServers": servers}, indent=2))
EOF

case $UI in
  heroui) ui_line="HeroUI v3 (@heroui/react). Look components up via the \`ui-kit\` MCP. No other component library." ;;
  shadcn) ui_line="shadcn/ui. Look components up via the \`ui-kit\` MCP. No other component library." ;;
  none)   ui_line="none chosen yet (no \`ui-kit\` MCP)" ;;
esac
if [ "$DB" = yes ]; then
  db_line="Supabase. Builders use the DEV project through the \`db\` MCP; reviewers use \`db-readonly\`.
  Every new table needs RLS policies. Schema changes only as migration files."
else db_line="none (no \`db\` MCP)"; fi
write_if_absent .factory/stack.md <<EOF
# Stack: $NAME (read by every factory agent)
- UI library: $ui_line
- DB/auth: $db_line
- Framework: TBD (decided in \`/factory init\`)
- Dev: TBD, must serve on http://localhost:3000 during review
- Gates: TBD (typecheck, lint, test, build commands)
- Risk hot spots: TBD
- UX testing: Playwright (real Chromium) through the \`ux-tester\` agent, black-box only.
EOF

touch .gitignore
for l in ".factory/jobs/" "node_modules/" ".env*" '!.env.example' ".claude/settings.local.json"; do
  grep -qxF "$l" .gitignore || echo "$l" >> .gitignore
done

# Pre-approve the project's MCP servers: Remote Control sessions can't show the approval prompt.
python3 - <<'EOF'
import json, os
p = ".claude/settings.local.json"
d = json.load(open(p)) if os.path.exists(p) else {}
names = set(json.load(open(".mcp.json")).get("mcpServers", {}))
d["enabledMcpjsonServers"] = sorted(set(d.get("enabledMcpjsonServers", [])) | names)
json.dump(d, open(p, "w"), indent=2); open(p, "a").write("\n")
EOF
echo "approved MCP servers: $(python3 -c 'import json; print(", ".join(json.load(open(".mcp.json"))["mcpServers"]) or "none")')"

SECRETS="$HOME/.config/claude-rc/$NAME.env"
mkdir -p "$(dirname "$SECRETS")" && chmod 700 "$(dirname "$SECRETS")"
[ -e "$SECRETS" ] || { install -m600 /dev/null "$SECRETS"; echo "created $SECRETS (empty, chmod 600)"; }

if [ "$NEW_REPO" = yes ]; then
  git add .claude/settings.json .mcp.json .factory/stack.md .gitignore
  if git commit -qm "Set up factory pipeline for $NAME" 2>/dev/null; then echo "committed initial pipeline files"
  else echo "NOT committed: set your git identity (git config --global user.name/user.email), then: cd $DIR && git commit -m 'Set up factory pipeline'"; fi
fi

cat <<EOF

Next steps for $NAME:
EOF
n=1
if [ "$DB" = yes ] && ! grep -q '^SUPABASE_ACCESS_TOKEN=' "$SECRETS"; then
  cat <<EOF
  $n. Save a Supabase access token (supabase.com/dashboard/account/tokens); input is hidden:
     read -rsp "Supabase token: " T && printf 'SUPABASE_ACCESS_TOKEN=%s\n' "\$T" >> $SECRETS && unset T && echo saved
EOF
  n=$((n+1))
fi
cat <<EOF
  $n. First run by hand (trust the folder, enable Remote Control, pick 2 = worktree), then Ctrl+C:
     cd $DIR && claude remote-control
  $((n+1)). Keep it running for good:
     systemctl --user enable --now claude-rc@$NAME
  $((n+2)). On your phone: Claude app > Code > "$NAME", then run /factory init
EOF
