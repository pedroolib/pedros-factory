---
name: setup
description: One-time machine setup for Pedro's Factory - checks dependencies, installs rtk, and optionally turns on Remote Control for every session.
disable-model-invocation: true
---
Set up THIS machine. Nothing here is project-specific.

1. Check the core dependencies with `claude plugin list`: `factory`, `mattpocock-skills` and
   `ponytail` from `pedros-factory` must be enabled. If one is missing, run
   `claude plugin install <name>@pedros-factory` after telling the user.
2. Ask ONE AskUserQuestion (multiSelect) with these options, explaining each in a sentence:
   - **rtk token saver** (recommended): compresses shell output for every agent. Installs the
     `rtk-oss` binary into `~/.local/bin` and a global Bash hook.
   - **Live dashboard** (recommended): `factory-dash`, a terminal view of every project's jobs
     and agent tree, live. Installs one binary into `~/.local/bin`; run it in any terminal.
   - **Remote Control for every session**: lets the Claude app on your phone reach sessions on
     this machine while it is awake.
   - **Git identity**: only offered when `git config --global user.email` is empty; builders
     and the approver commit, so it is needed.
3. For each choice:
   - rtk: run `bash "${CLAUDE_PLUGIN_ROOT}/scripts/install-rtk.sh"`. It prints the next command
     (`rtk init --global --auto-patch`); run that too. If rtk is already installed at the pinned
     version, say so and skip. Native Windows isn't supported: tell the user to use WSL.
   - Live dashboard: run `bash "${CLAUDE_PLUGIN_ROOT}/scripts/install-dash.sh"`. Tell the user
     to run `factory-dash` in a separate terminal (it can't render inside this session), and
     pass on any PATH note the script prints.
   - Remote Control: set `"remoteControlAtStartup": true` in `~/.claude/settings.json` (merge
     with the existing JSON, keep everything else).
   - Git identity: ask for name and email, then run `git config --global user.name ...` and
     `git config --global user.email ...`.
4. Report what changed and that new sessions pick it up after a restart of Claude Code.
5. Point to the next step: open a project and run `/factory:init`.
   If this machine should run sessions 24/7 (a server), mention the server add-on in the
   README: `server/bootstrap.sh`.
