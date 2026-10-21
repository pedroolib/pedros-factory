<p align="center">
  <img src="https://raw.githubusercontent.com/pedroolib/pedros-factory/assets/banner.svg" alt="Pedro's Factory — an orchestrated software-delivery pipeline for Claude Code" width="100%">
</p>

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Claude Code plugin marketplace](https://img.shields.io/badge/Claude%20Code-plugin%20marketplace-orange)
![Platforms](https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20WSL-lightgrey)

Describe a feature to [Claude Code](https://claude.com/claude-code); the factory specifies it, builds it in parallel, verifies it independently
and merges it, or hands the decision back to you.

```
/factory:build add CSV export to the reports page
```

- **Specification first.** An Opus orchestrator interviews you until nothing expensive is left unknown.
- **Parallel, isolated builds.** Each work package gets its own git worktree and owns only its files.
- **Independent verification.** Deterministic gates, code and security reviewers, and a
  **black-box tester that never sees the source code**.
- **Human control where it matters.** Migrations, auth, payments, hardware and safety limits
  always escalate to you.
- **Domain profiles.** Web, robotics, firmware and data/ML, each with its own gates, testers and reviewers.
- **Runs anywhere.** A workstation, or an always-on server you drive from the Claude mobile app.

**Contents:** [How it works](#how-it-works) · [Plugins](#plugins) · [Requirements](#requirements) ·
[Installation](#installation) · [Project setup](#project-setup) · [Usage](#usage) ·
[Dashboard](#live-dashboard) · [Server mode](#server-mode) · [Extending](#extending-add-a-profile) ·
[Security and limitations](#security-and-limitations) · [Troubleshooting](#troubleshooting) ·
[Contributing](#contributing) · [License](#license)

## How it works

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

Job files (spec, tickets, reviews, decision) live in `.factory/jobs/<job-id>/`. Lessons from
finished jobs accumulate in `.factory/LESSONS.md`, and builders read them on every job.

**What always comes back to you:** anything a profile marks as risky. Database migrations, auth
and payments (web); real hardware, safety limits and e-stop code (robotics); flashing a board
that isn't on your safe list (firmware); changes to data splits or metrics (data). Plus jobs over
600 changed lines or that hit the review-round limit.

## Plugins

The repository is a Claude Code plugin marketplace named `pedros-factory`. The core plugin is
installed once per machine; each project enables the profiles it needs.

| Plugin | Agents | Black-box tester | Bundled MCP |
|---|---|---|---|
| `factory` (core) | scout (Haiku), builder (Sonnet), reviewer-code, reviewer-security, approver (Opus) | `user-tester`: runs your CLI/API/library from the docs only | Context7 |
| `factory-webdev` | designer, reviewer-ui | `ux-tester`: drives a real browser from a URL, persona and goal | Playwright, Figma |
| `factory-robotics` | reviewer-safety | `sim-tester`: runs Gazebo headless, judges ROS 2 topics, services and logs | none |
| `factory-firmware` | reviewer-firmware | `fw-tester`: PlatformIO native tests, simulator, or a connected board from your safe list | none |
| `factory-data` | reviewer-data | `data-tester`: smoke-trains on sample data, GPU if present, checks reproducibility | none |

Profiles can be combined, for example robotics + webdev for a robot with a web dashboard.

Dependencies install automatically and are pinned to exact commits in
[`marketplace.json`](.claude-plugin/marketplace.json): core pulls
[Matt Pocock's skills](https://github.com/mattpocock/skills) and
[Ponytail](https://github.com/DietrichGebert/ponytail); the web profile adds
[frontend-design](https://github.com/anthropics/claude-plugins-official),
[UI UX Pro Max](https://github.com/nextlevelbuilder/ui-ux-pro-max-skill) and
[Impeccable](https://github.com/pbakaus/impeccable).

## Requirements

- Claude Code with a Claude Pro or Max plan. On Pro, expect roughly 1 to 3 features per 5-hour
  window; profiles with more testers cost more.
- `git`, and Node.js 18+ for the bundled MCP servers. Linux, macOS, or Windows through WSL.
- Per profile: Node.js 22+ for HeroUI's MCP (webdev), ROS 2 + Gazebo (robotics), PlatformIO
  (firmware), your Python environment (data). `/factory:init` checks and reports what is missing.

## Installation

Once per machine, in Claude Code:

```
/plugin marketplace add pedroolib/pedros-factory
/plugin install factory@pedros-factory
/factory:setup
```

Or from a shell:

```bash
claude plugin marketplace add pedroolib/pedros-factory && claude plugin install factory@pedros-factory
```

`/factory:setup` asks once and then optionally installs:

- **rtk**, a pinned, checksum-verified binary plus a hook that compresses shell output for every agent;
- **factory-dash**, the [terminal dashboard](#live-dashboard);
- **Remote Control** for every session, so the Claude app on your phone can reach this machine;
- your git identity.

## Project setup

Open Claude Code in the project folder and run:

```
/factory:init
```

1. **Profiles.** It detects likely profiles from your files, asks you to confirm, and installs
   them for this project only (`--scope project`), so a robotics repo never loads web design
   skills. It then asks you to run `/reload-plugins` and `/factory:init` again.
2. **Questions.** Each profile asks what it needs: UI library and database for web, ROS distro
   and sim launch command for robotics, safe dev boards for firmware, train and eval commands
   for data. Answers go to `.factory/stack.md`, which every agent reads.
3. **MCP servers.** It writes the project's `.mcp.json` (for example `ui-kit` for HeroUI or
   shadcn, `db` and `db-readonly` for Supabase), pre-approves the servers, and walks you through
   each account one step at a time:
   - with a browser: `/mcp`, pick the server, Authenticate;
   - on a server, or for services without browser sign-in: it tells you where to create a token
     and gives you one command that saves it to `~/.config/claude-rc/<project>.env` with hidden
     input. Tokens never go through the chat.

   It then runs `claude mcp list` and does not finish until every server shows Connected.
4. **Commit.** It offers to commit `.claude/settings.json`, `.mcp.json`, `.factory/stack.md`
   and `.gitignore`, so anyone who clones the repository gets the same profiles offered.

## Usage

| Command | Description |
|---|---|
| `/factory:build <feature>` | Run a feature through the whole pipeline |
| `/factory:status` | Table of all jobs and their stage |
| `/factory:approve <job>` | Merge a job the approver escalated to you |
| `/factory:rework <job> <note>` | Send a job back to its builders with your note |
| `/factory:doctor` | Check machine and project, print the exact fix for anything missing |
| `/factory:init` | Re-run any time to add a profile or fix MCP setup |
| `/factory:remove` | Remove the factory from this project (asks first, keeps your code); `--everything` uninstalls it from this machine |

## Live dashboard

`dashboard/` is a Rust terminal UI ([ratatui](https://ratatui.rs), Everforest colors) with one
tab per project. Each orchestrator session from the last hour is drawn as a tree: the
orchestrator, then every agent it spawned, with model, task, status (running, needs you,
blocked, idle, done), cost and time. A side panel shows the project's jobs and a live feed of the
selected agent's tool calls.

`/factory:setup` and `server/bootstrap.sh` install a prebuilt, checksum-verified binary into
`~/.local/bin` for Linux and macOS; Rust is not required. Then, in any terminal:

```bash
factory-dash                              # your pinned projects as tabs
factory-dash ~/Projects/app ~/work/api    # just these, for this run
```

| Key | Action |
|---|---|
| `h` / `l` | Switch project |
| `j` / `k` | Move between agents |
| `Enter` | Fold a subtree (or expand a log entry) |
| `g` / `G` | Top / bottom |
| `Tab` | Move to the activity log |
| `a` | Project picker: every folder with a `.factory/` up to 3 levels under `$HOME`; `Enter` pins or unpins |
| `r` | Rescan |
| `q` | Quit |

Pinned projects are saved in `~/.config/factory-dash/projects`, one path per line, and can be
edited by hand. Until you pin anything, tabs are the projects with agent activity in the last
hour; once you unpin everything, no tabs are open. Costs are estimates at API list prices, read
from the transcripts in `~/.claude/projects`. A Pro or Max plan is not billed this way.

## Server mode

A spare Linux machine (homelab, mini PC, VPS) can host your projects so they keep working while
your laptop is off, and you drive them from the Claude app.

```bash
git clone https://github.com/pedroolib/pedros-factory.git ~/pedros-factory && ~/pedros-factory/server/bootstrap.sh
```

The bootstrap needs no sudo. It installs Node.js 22, Claude Code, rtk, the dashboard and the
`factory` plugin in your home folder, plus a `claude-rc@<project>` systemd user service that
keeps one Remote Control server per project running in tmux. It prints anything that needs root
(usually `sudo loginctl enable-linger $USER`). Then, per project in `~/projects/<name>`:

1. Run `claude` in the folder: trust it, `/login` if needed, run `/factory:init`. On a server,
   init uses tokens in `~/.config/claude-rc/<name>.env` instead of browser sign-in.
2. Run `claude remote-control` once: enable it, choose `2` (worktree), then Ctrl+C.
3. `systemctl --user enable --now claude-rc@<name>`
4. On your phone: Claude app > Code > open `<name>`.

## Updating

```
/plugin marketplace update pedros-factory
```

Dependency versions only move when this repository bumps the pinned commits, so every machine
gets the same versions. On a server, also run `git pull` in `~/pedros-factory`.

## Extending: add a profile

A profile is a plugin in `plugins/factory-<name>/` containing its agents and one hidden skill,
`skills/profile/SKILL.md`, with the following sections, which `/factory:init` and
`/factory:build` read:

| Section | Used by | Contents |
|---|---|---|
| INIT | `/factory:init` | questions, what to record in stack.md, MCP entries, secrets, tooling checks |
| BUILD | build stages 2b and 3 | checkpoints before building, rules added to every builder brief |
| GATES | build stage 5 | deterministic commands |
| TESTERS | build stage 6 | black-box tester agents and exactly which inputs they get |
| REVIEWERS | build stage 6 | extra reviewer agents |
| ESCALATE | approver | risk tags that always go to a human |

Add the plugin to `marketplace.json` and to the detection table in
`plugins/factory/skills/init/SKILL.md`.

## Security and limitations

- **Early software.** Interfaces, file layouts and skill behavior may change between releases.
- **Cost.** Every job uses several model sessions; plan limits apply (see [Requirements](#requirements)).
- **Black-box isolation is advisory for shell-based testers.** The CLI, simulation, firmware and
  data testers need Bash to run your software, so "never read the source" is an instruction, not
  a hard wall. The web UX tester has no shell or file tools at all.
- **Human gate.** Risky changes are never merged automatically. Review escalations rather than
  approving by habit.
- **Credentials.** Tokens are stored outside the repository in `~/.config/claude-rc/` and never
  pass through the chat. Dependencies are pinned to exact commits; review a bump before updating.
- **Reporting vulnerabilities.** Please report security issues privately to the repository owner,
  not in a public issue.

## Troubleshooting

Run `/factory:doctor` first; it checks the machine and project and prints the exact fix.

- **`/mcp` in the Claude app does not list `db` or `ui-kit`.** The app's menu only shows its own
  connectors. `/factory:doctor` checks the project's servers.
- **The app shows only GitHub and browser tools.** You opened a cloud session. Open the session
  named after your project, which runs on your machine.
- **"Failed to resolve base branch HEAD".** The repository has no commits and builders need one
  for their worktrees: `git commit --allow-empty -m "Initial commit"`.
- **A server service keeps restarting.** Run `journalctl --user -u claude-rc@<name> -n 30` and
  `tmux attach -t claude-rc-<name>`. "Already served by a terminal" means a manual
  `claude remote-control` is still running in that folder.

## Uninstall

One project: run `/factory:remove` in it. It removes the profile plugins from that project, plus
the `stack.md`, settings, server approvals and `.gitignore` lines that init added. It never
touches your code. Unless you tick them, it keeps your job history, the MCP servers in `.mcp.json`
(your app may use them) and your token file. On a server it prints the command to stop
`claude-rc@<project>` instead of stopping the session you are in.

Everything on this machine: `/factory:remove --everything` removes the marketplace and all its
plugins, and optionally rtk and the Remote Control setting. Both list what they will remove and
ask first. By hand:

```
/plugin marketplace remove pedros-factory
```

This removes the marketplace and every plugin installed from it. For rtk, delete
`~/.local/bin/rtk` and its hook in `~/.claude/settings.json`. On a server:
`systemctl --user disable --now 'claude-rc@*'`.

## Contributing

Issues and pull requests are welcome. Keep changes focused and describe the behavior change and
how you verified it. For a new profile, follow [Extending](#extending-add-a-profile); run
`/factory:doctor` against a scratch project before opening a pull request.

**Dashboard development.** Build locally with `cargo install --path dashboard`. To release, bump
`version` in `dashboard/Cargo.toml` and `DASH_VERSION` in
`plugins/factory/scripts/install-dash.sh`, then push a tag `dash-v<version>`;
`.github/workflows/dashboard-release.yml` builds and publishes the binaries.

## License

[MIT](LICENSE) for this repository. Each dependency keeps its own license.
