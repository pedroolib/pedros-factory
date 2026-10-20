# Pedro's Factory

A software factory for Claude Code. You describe a feature to one Opus orchestrator. It
questions you until the spec is solid, runs parallel builders, checks the result with
deterministic gates, reviewers and a **black-box tester that never sees the code**, then
merges or asks you. It works for web apps, robotics, firmware, data/ML and plain code, on
Linux, macOS and WSL, on a laptop or on an always-on server you reach from your phone.

```
/factory:build <feature>
  1  grill      Opus (in your session)   asks until nothing expensive is left unknown
  2  spec       Opus                     spec.md + tickets.md (work packages, owned files)
  2b checkpoint profile-specific         e.g. web design: prototype + screenshots -> you pick
  3  build      builder x N   Sonnet     one git worktree each; Haiku scouts explore the code
  4  questions  builders stop BLOCKED -> Opus answers, or asks you once in a batch
  5  gates      typecheck, lint, tests, build + profile gates (no LLM cost)
  6  review     code + security reviewers, profile reviewers, black-box testers (max 2 rounds)
  7  approve    approver      Opus       APPROVE -> rebase + merge, ESCALATE -> you
```

## Plugins

The repo is a Claude Code plugin marketplace called `pedros-factory`. Core is installed once per
machine; each project turns on the profiles it needs.

| Plugin | Agents | Black-box tester | Bundled MCP |
|---|---|---|---|
| `factory` (core) | scout (Haiku), builder (Sonnet), reviewer-code, reviewer-security, approver (Opus) | `user-tester`: runs your CLI/API/library from the docs only | Context7 |
| `factory-webdev` | designer, reviewer-ui | `ux-tester`: drives a real browser from a URL, persona and goal | Playwright, Figma |
| `factory-robotics` | reviewer-safety | `sim-tester`: runs Gazebo headless, judges ROS 2 topics, services and logs | none |
| `factory-firmware` | reviewer-firmware | `fw-tester`: PlatformIO native tests, simulator, or a connected board from your safe list | none |
| `factory-data` | reviewer-data | `data-tester`: smoke-trains on sample data, GPU if present, checks reproducibility | none |

Dependencies install automatically and are pinned to exact commits in
[`marketplace.json`](.claude-plugin/marketplace.json): core pulls
[Matt Pocock's skills](https://github.com/mattpocock/skills) and
[Ponytail](https://github.com/DietrichGebert/ponytail); the web profile adds
[frontend-design](https://github.com/anthropics/claude-plugins-official),
[UI UX Pro Max](https://github.com/nextlevelbuilder/ui-ux-pro-max-skill) and
[Impeccable](https://github.com/pbakaus/impeccable). Profiles can be combined, for example
robotics + webdev for a robot with a web dashboard.

## Requirements

- Claude Code with a Claude Pro or Max plan. On Pro, expect roughly 1 to 3 features per 5-hour
  window; profiles with more testers cost more.
- `git`, and Node.js 18+ for the bundled MCP servers. Linux, macOS, or Windows through WSL.
- Per profile: Node.js 22+ for HeroUI's MCP (webdev), ROS 2 + Gazebo (robotics), PlatformIO
  (firmware), your Python environment (data). `/factory:init` checks and tells you what's missing.

## Install (once per machine)

In Claude Code:

```
/plugin marketplace add pedroolib/pedros-factory
```
```
/plugin install factory@pedros-factory
```
```
/factory:setup
```

The same from a shell:

```bash
claude plugin marketplace add pedroolib/pedros-factory && claude plugin install factory@pedros-factory
```

`/factory:setup` asks once and then optionally installs rtk (a pinned, checksum-verified binary
plus a hook that compresses shell output for every agent), turns on Remote Control for every
session so the Claude app on your phone can reach this machine, and sets your git identity.

## Set up a project

Open Claude Code in the project folder and run:

```
/factory:init
```

1. **Profiles.** It detects likely profiles from your files, asks you to confirm, and installs
   those profile plugins for this project only (`--scope project`), so a robotics repo never
   loads web design skills. Then it asks you to run `/reload-plugins` and `/factory:init` again.
2. **Questions.** Each profile asks what it needs: UI library and database for web, ROS distro
   and sim launch command for robotics, safe dev boards for firmware, train and eval commands
   for data. Answers go to `.factory/stack.md`, which every agent reads.
3. **MCP servers, step by step.** It writes the project's `.mcp.json` (for example `ui-kit` for
   HeroUI or shadcn, `db` and `db-readonly` for Supabase), pre-approves them, and walks you
   through each account one step at a time:
   - on a computer with a browser: `/mcp`, pick the server, Authenticate;
   - on a server, or for services without browser sign-in: it tells you where to create a
     token and gives you one command that saves it to `~/.config/claude-rc/<project>.env`
     with hidden input. Tokens never go through the chat.

   Then it runs `claude mcp list` and doesn't finish until every server shows Connected.
4. **Commit.** It offers to commit `.claude/settings.json`, `.mcp.json`, `.factory/stack.md`
   and `.gitignore`. Anyone who clones the repo gets the same profiles offered to them.

## Use it

| Command | What it does |
|---|---|
| `/factory:build <feature>` | runs a feature through the whole pipeline |
| `/factory:status` | table of all jobs and their stage |
| `/factory:approve <job>` | merges a job the approver escalated to you |
| `/factory:rework <job> <note>` | sends a job back to its builders with your note |
| `/factory:doctor` | checks machine and project, prints the exact fix for anything missing |
| `/factory:remove` | removes the factory from this project (asks first, keeps your code); `--everything` uninstalls it from this machine |
| `/factory:init` | re-run any time to add a profile or fix MCP setup |

Job files (spec, tickets, reviews, decision) live in `.factory/jobs/<job-id>/`. Lessons from
finished jobs accumulate in `.factory/LESSONS.md`, and builders read them on every job.

**What always comes back to you:** anything a profile marks as risky. Database migrations, auth
and payments (web); real hardware, safety limits and e-stop code (robotics); flashing a board
that isn't on your safe list (firmware); changes to data splits or metrics (data). Plus jobs over
600 changed lines or that hit the review-round limit.

## Optional: run it 24/7 on a server

A spare Linux machine (a homelab, a mini PC, a VPS) can host your projects so they keep working
while your laptop is off, and you drive them from the Claude app.

```bash
git clone https://github.com/pedroolib/pedros-factory.git ~/pedros-factory && ~/pedros-factory/server/bootstrap.sh
```

The bootstrap needs no sudo. It installs Node.js 22, Claude Code, rtk and the `factory` plugin
in your home folder, plus a `claude-rc@<project>` systemd user service that keeps one Remote
Control server per project running in tmux. It prints anything that needs root (usually
`sudo loginctl enable-linger $USER`). Then, per project in `~/projects/<name>`:

1. `claude` in the folder: trust it, `/login` if needed, run `/factory:init`. On a server,
   init uses tokens in `~/.config/claude-rc/<name>.env` instead of browser sign-in.
2. `claude remote-control` once: enable it, choose `2` (worktree), then Ctrl+C.
3. `systemctl --user enable --now claude-rc@<name>`
4. On your phone: Claude app > Code > open `<name>`.

## Update

```
/plugin marketplace update pedros-factory
```

Dependency versions only move when this repo bumps the pinned commits, so every machine gets the
same versions. On a server, also `git pull` in `~/pedros-factory`.

## Add a profile

A profile is a plugin in `plugins/factory-<name>/` with its agents and one hidden skill,
`skills/profile/SKILL.md`, with these sections, which `/factory:init` and `/factory:build` read:

| Section | Used by | Contains |
|---|---|---|
| INIT | `/factory:init` | questions, what to record in stack.md, MCP entries, secrets, tooling checks |
| BUILD | build stages 2b and 3 | checkpoints before building, rules added to every builder brief |
| GATES | build stage 5 | deterministic commands |
| TESTERS | build stage 6 | black-box tester agents and exactly which inputs they get |
| REVIEWERS | build stage 6 | extra reviewer agents |
| ESCALATE | approver | risk tags that always go to a human |

Add the plugin to `marketplace.json` and to the detection table in
`plugins/factory/skills/init/SKILL.md`.

## Troubleshooting

- **`/mcp` in the Claude app doesn't list `db` or `ui-kit`.** The app's menu only shows its own
  connectors. Run `/factory:doctor` in the session; it checks the project's servers.
- **The app shows only GitHub and browser tools.** You opened a cloud session. Open the session
  named after your project, which runs on your machine.
- **"Failed to resolve base branch HEAD".** The repository has no commits; builders need one
  for their worktrees. `git commit --allow-empty -m "Initial commit"`.
- **A server service keeps restarting.** `journalctl --user -u claude-rc@<name> -n 30` and
  `tmux attach -t claude-rc-<name>`. "Already served by a terminal" means a manual
  `claude remote-control` is still running in that folder.
- **Black-box testers with shell access.** The CLI, simulation, firmware and data testers need
  Bash to run your software, so "never read the source" is an instruction, not a hard wall.
  The web UX tester has no shell or file tools at all.

## Uninstall

One project: `/factory:remove` in it. Everything on this machine: `/factory:remove --everything`.
Both list what they'll remove and ask first. By hand:

```
/plugin marketplace remove pedros-factory
```

This removes the marketplace and every plugin installed from it. rtk: delete `~/.local/bin/rtk`
and its hook in `~/.claude/settings.json`. Server: `systemctl --user disable --now 'claude-rc@*'`.

## License

MIT for this repository. Each dependency keeps its own license.
