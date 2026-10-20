---
name: doctor
description: Check that Pedro's Factory is healthy on this machine and in this project, and print the exact fix for anything missing.
---
Run these checks with Bash and print a table `check | status | fix`. Change nothing.

Machine:
- `claude plugin list`: `factory`, `mattpocock-skills`, `ponytail` enabled.
- `rtk --version` and the `rtk hook claude` entry in `~/.claude/settings.json` (optional).
- `git config --global user.email` is set.

Project (skip if `.factory/stack.md` is missing; then say "run `/factory:init`"):
- Every profile under `Profiles:` in stack.md has `factory-<profile>` enabled for this project
  (fix: `claude plugin install factory-<profile>@pedros-factory --scope project`, then
  `/reload-plugins`).
- Each profile's dependencies are enabled (from `claude plugin details factory-<profile>`).
- `claude mcp list` (on a server, prefix `set -a; . ~/.config/claude-rc/<project>.env
  2>/dev/null; set +a;`): every server in `.mcp.json` shows Connected, and each one is listed in
  `enabledMcpjsonServers` in `.claude/settings.local.json`.
- Each profile skill's INIT tooling is present (for example the Playwright browser for webdev,
  `gz` and `ros2` for robotics, `pio` for firmware).
- `.gitignore` contains `.factory/jobs/`.
- The repository has at least one commit (`git rev-parse HEAD`); builders need it for worktrees.

End with the single most important fix first.
