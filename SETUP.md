# Setup

## Global (once per machine: homelab and laptop)
Keep this folder in its own git repo, then:
```bash
./install.sh                       # symlinks global/agents + global/skills into ~/.claude
rtk init --global && rtk gain      # token saver hook; `gain` proves it's Rust Token Killer
claude plugin marketplace add mattpocock/skills && claude plugin install mattpocock-skills@mattpocock
claude plugin install frontend-design@anthropic-plugin-directory
claude plugin install figma@anthropic-plugin-directory   # Dev Mode MCP needs a paid Figma seat
```
Inside `claude`, as two separate prompts:
```
/plugin marketplace add DietrichGebert/ponytail
/plugin install ponytail@ponytail
```
Install UI UX Pro Max and Impeccable from their repos and confirm the skill names match
`ui-ux-pro-max` / `impeccable` in designer.md and reviewer-ui.md.
Do NOT also install Superpowers (it competes with Pocock's grill/tdd flow).

## Per project
1. Copy `project-template/` into the repo root (`.mcp.json`, `.factory/stack.md`, `.claude/settings.json`).
2. Edit `.mcp.json`: keep the server NAMES (`ui-kit`, `db`, `db-readonly`), change what they point to.
   - shadcn project:  `"ui-kit": {"command": "npx", "args": ["-y", "shadcn@latest", "mcp"]}`
   - no UI library:   delete `ui-kit`
   - no Supabase:     delete `db` / `db-readonly`, or point them at another DB MCP
3. Edit `.factory/stack.md`, open `claude` once, trust the folder, log in to the MCP servers.
4. `/factory init` checks everything; then `/factory <feature>`.
5. To change one agent for one project only, copy it into `<repo>/.claude/agents/` with the
   same name: project agents override the global ones.
