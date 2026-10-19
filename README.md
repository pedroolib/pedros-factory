# pipeline-kit

A Claude Code "software factory" that runs on your own Linux server and that you drive from the
Claude mobile app. You describe a feature to one Opus orchestrator. It grills you, writes a spec,
runs parallel builders, checks the result with reviewers and a black-box UX tester, then merges
or asks you.

```
/factory <feature>
  1  grill      Opus (you, in the session)    asks until the spec has no expensive unknowns
  2  spec       Opus                          spec.md + tickets.md (work packages, owned files)
  2b design     designer       Sonnet         tokens, prototype, screenshots -> you approve
  3  build      builder x N    Sonnet         one git worktree each; scouts (Haiku) explore
  4  questions  builders stop with BLOCKED -> Opus answers or asks you once, in a batch
  5  gates      typecheck, lint, tests, build (no LLM cost)
  6  review     reviewer-security, reviewer-code, reviewer-ui, ux-tester  (max 2 rework rounds)
  7  approve    approver       Opus           APPROVE -> rebase + merge, ESCALATE -> your phone
```

| Agent | Model | Preloaded skills | MCP access |
|---|---|---|---|
| orchestrator (`/factory`) | Opus | Pocock grill-with-docs, to-tickets, retro | project servers |
| builder | Sonnet | tdd, codebase-design, ponytail, frontend-design | db, ui-kit, figma, context7 |
| scout | Haiku | none | none (Read/Glob/Grep only) |
| designer | Sonnet | frontend-design, ui-ux-pro-max | ui-kit, figma, playwright |
| reviewer-security | Sonnet, Opus on risky jobs | none | db-readonly |
| reviewer-code | Sonnet | code-review, ponytail-review | none |
| reviewer-ui | Sonnet | impeccable | playwright, figma (no code access) |
| ux-tester | Haiku | none | playwright only, locked to localhost:3000 |
| approver | Opus | none | none, read-only |

The pipeline (agents, skill, plugins) is **global**: it lives in `~/.claude` and every project
sees it. What differs per project (UI kit, database) lives **in the project**: its `.mcp.json`
names the servers `ui-kit`, `db` and `db-readonly`, and the agents only ever use those names.

## Requirements

- Linux x86_64 with systemd (tested on Ubuntu 24.04). An always-on homelab server is the point.
- A Claude Pro or Max plan. On Pro, expect roughly 1 to 3 features per 5-hour window.
- `git`, `curl`, `python3`, `tmux` (the bootstrap tells you if one is missing).
- Optional: a Supabase project per app, a paid Figma seat for the Figma MCP.

## Install (once per machine)

The repo is private, so clone it with the GitHub CLI (`gh auth login` first) or an SSH key:

```bash
gh repo clone pedroolib/pipeline-kit ~/pipeline-kit
```
```bash
~/pipeline-kit/bootstrap.sh
```

The bootstrap needs no sudo and installs, in your home folder:

- Node.js 22 (`~/.local/node`), Claude Code, and [rtk-oss](https://github.com/rtk-ai/rtk) 1.0.1,
  with its global Bash hook that compresses command output
- the plugins in [`plugins.txt`](plugins.txt): Matt Pocock's skills, Ponytail, frontend-design,
  Figma, UI UX Pro Max, Impeccable
- Playwright's Chromium, at the version `@playwright/mcp` expects
- symlinks from `~/.claude/agents` and `~/.claude/skills/factory` to this repo
- the `claude-rc@.service` systemd user unit

At the end it prints what is left for you. On a fresh machine that is usually:

```bash
sudo npx -y playwright@<version> install-deps chromium   # exact line is printed; or: sudo apt-get install -y libasound2t64
```
```bash
sudo loginctl enable-linger $USER
```
```bash
claude
```
In `claude`: run `/login`, then `/exit`. Then run `~/pipeline-kit/bootstrap.sh` again: it should end with "Nothing left to do".

## Add a project

```bash
~/pipeline-kit/new-project.sh myapp --supabase-ref abcdefghijklmnopqrst
```

| Flag | Default | Meaning |
|---|---|---|
| `--ui heroui\|shadcn\|none` | `heroui` | which component library the `ui-kit` MCP serves |
| `--supabase-ref <ref>` | required unless `--no-db` | the ID in `supabase.com/dashboard/project/<ref>` |
| `--no-db` | | no `db` / `db-readonly` servers |

It works on a new or an existing `~/projects/<name>` and never overwrites existing files. It writes
`.mcp.json`, `.factory/stack.md`, `.claude/settings.json`, updates `.gitignore`, pre-approves the
project's MCP servers, and creates the secrets file `~/.config/claude-rc/<name>.env` (chmod 600).

Then follow the steps it prints:

1. **Supabase token** (only with a database). Create one at
   [supabase.com/dashboard/account/tokens](https://supabase.com/dashboard/account/tokens) and save
   it with the printed `read -rsp ...` line. The input is hidden and stays out of shell history.
   A token is used instead of a browser login because OAuth can't finish on a headless server.
2. **First run by hand:** `cd ~/projects/<name> && claude remote-control`. Trust the folder,
   enable Remote Control, choose `2` (worktree), then press Ctrl+C.
3. **Run it for good:** `systemctl --user enable --now claude-rc@<name>`
4. **On your phone:** Claude app > Code > open the `<name>` session, then run `/factory init`.

## Use it

| Command | What it does |
|---|---|
| `/factory init` | grills you about the stack and fills in `.factory/stack.md` |
| `/factory <feature>` | runs a feature through the whole pipeline |
| `/factory status` | table of all jobs and their stage |
| `/factory approve <job>` | merge a job the approver escalated to you |
| `/factory rework <job> <note>` | send a job back to its builders with your note |

Job files (spec, tickets, reviews, decision) are in `.factory/jobs/<job-id>/` inside the project.
Lessons from finished jobs accumulate in `.factory/LESSONS.md`, and every builder reads it.

## Update

```bash
cd ~/pipeline-kit && git pull && ./bootstrap.sh
```

Changes to agents and the skill apply at once (they are symlinks). Restart running servers to
pick up plugin updates: `systemctl --user restart claude-rc@<name>`.

## Customize

- **One agent, one project:** copy it to `<project>/.claude/agents/<agent>.md` and edit it. A
  project agent overrides the global one with the same name.
- **Another UI kit or database:** edit the project's `.mcp.json`, but keep the server names
  `ui-kit`, `db`, `db-readonly`. Update `.factory/stack.md` to match.
- **Add a plugin for everyone:** add a line to `plugins.txt`, then re-run `bootstrap.sh`.
- **Port:** the review agents' Playwright is locked to `http://localhost:3000`. To change it,
  edit `--allowed-origins` in `global/agents/{ux-tester,reviewer-ui}.md`.

## Layout

```
bootstrap.sh              machine setup and update (idempotent, no sudo)
new-project.sh            add the pipeline to ~/projects/<name>
plugins.txt               plugins + marketplaces the agents depend on
global/agents/*.md        the 8 agents, linked into ~/.claude/agents
global/skills/factory/    the /factory orchestrator skill, linked into ~/.claude/skills
project-template/         .claude/settings.json copied into each project (Opus, spawn depth 2)
deploy/claude-rc@.service systemd user unit: one Remote Control server per project, in tmux
scripts/link.sh           symlinks global/ into ~/.claude (called by bootstrap.sh)
```

## Troubleshooting

- **`/mcp` in the app doesn't list `db` or `ui-kit`.** Expected: the app's menu only shows its
  own connectors. The servers run on the server. Check there with
  `cd ~/projects/<name> && set -a && . ~/.config/claude-rc/<name>.env && set +a && claude mcp list`.
- **The app shows only GitHub/browser tools.** You opened a cloud session. Open the session named
  after your project (it runs on your server) instead of starting a new one.
- **Service keeps restarting.** `journalctl --user -u claude-rc@<name> -n 30` and
  `tmux attach -t claude-rc-<name>`. "Already served by a terminal" means a manual
  `claude remote-control` is still running in that folder: stop it.
- **"Skill listing over budget" in debug logs.** Too many skills installed; descriptions get
  truncated. Disable unused ones with `/skills`.
- **A subagent can't find a skill.** Plugin skills are namespaced (`mattpocock-skills:tdd`).
  Pocock's user-only skills (`grill-with-docs`, `to-tickets`, `retro`) can't be invoked by the
  model, so `/factory` reads their `SKILL.md` files directly.

## Uninstall

```bash
systemctl --user disable --now 'claude-rc@*'
```
```bash
rm ~/.config/systemd/user/claude-rc@.service ~/.claude/agents/{approver,builder,designer,reviewer-code,reviewer-security,reviewer-ui,scout,ux-tester}.md ~/.claude/skills/factory
```

Plugins: `claude plugin uninstall <plugin>` for each line in `plugins.txt`. rtk: remove its hook
from `~/.claude/settings.json` and delete `~/.local/bin/rtk`.
