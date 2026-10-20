---
name: init
description: Set up the current project for Pedro's Factory - pick profiles, install their plugins for this project, write stack.md, and set up and verify every MCP server step by step.
disable-model-invocation: true
---
You set up THIS project (the current working directory) for the factory. Work in two phases,
because plugins installed during a session only load after `/reload-plugins`.
Keep state in `.factory/init-state.json` so a second `/factory:init` resumes where you stopped.

## Phase A: profiles (skip if init-state.json says phase B)

1. If this folder isn't a git repository, ask whether to run `git init -b main`. Builders work
   in git worktrees, which need at least one commit: if the repository has none, say so and
   offer `git commit --allow-empty -m "Initial commit"`.
2. Detect likely profiles from the files:

   | Profile | Signals |
   |---|---|
   | webdev | package.json with a web framework (react, next, vue, svelte, astro, vite), index.html |
   | robotics | package.xml, ament/colcon/CMake ROS packages, *.launch.py, .world/.sdf files |
   | firmware | platformio.ini, src/main.cpp with Arduino.h, *.ino |
   | data | *.ipynb, requirements with torch/tensorflow/sklearn/pandas, data/ or datasets/ folders |
   | (general) | always on: CLIs, libraries, backends |

3. Ask with AskUserQuestion (multiSelect) which profiles apply, pre-checking the detected ones.
   An empty or new folder: ask what the project will be.
4. Show the plugins that will be installed for this project only (`factory-<profile>` and
   the dependencies it pulls in), then ask ONE confirmation. On yes, run for each profile:
   `claude plugin install factory-<profile>@pedros-factory --scope project`
5. Write `.factory/init-state.json` = `{"phase": "B", "profiles": [...]}` and tell the user:
   "Run `/reload-plugins`, then `/factory:init` again to finish." Stop here.
   If no profile beyond general was chosen, go straight to phase B.

## Phase B: configure (profiles loaded)

1. For every chosen profile, load `factory-<profile>:profile` with the Skill tool and read its
   INIT section. Ask the questions those sections list, plus the general ones below, in
   AskUserQuestion rounds of up to 4. Skip anything you can read from the repo.
   General: dev/run command, test, lint, typecheck and build commands, risky folders.
2. Write `.factory/stack.md` (never silently overwrite an existing one; show a diff and ask):
   ```
   # Stack: <project name>
   Profiles: <comma-separated list>
   Run: <command>  (web apps must serve on http://localhost:3000 during review)
   Gates:
   - <typecheck command>
   - <lint command>
   - <test command>
   - <build command>
   Always escalate:
   - <paths and change types from the profiles' ESCALATE sections and the user's answers>
   <one section per profile, with the facts its INIT section asks you to record>
   ```
3. Write `.claude/settings.json` (merge with any existing file):
   `"model": "opus"` and
   `"env": {"CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH": "2", "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "0"}`.
4. Add to `.gitignore` if missing: `.factory/jobs/`, `.factory/init-state.json`,
   `.claude/settings.local.json`.
5. Run each profile's INIT tooling steps (for example downloading a browser). Ask once before
   running anything that downloads or installs; print any command that needs sudo instead of
   running it.

## Phase C: MCP servers, one at a time

Collect the MCP entries the profiles' INIT sections ask for. Project-specific servers go in the
project's `.mcp.json` (merge, keep existing entries). Servers bundled inside a profile plugin
need no entry. Then:

1. Pre-approve every server you wrote: add its name to `enabledMcpjsonServers` in
   `.claude/settings.local.json`. Remote sessions can't show the approval prompt, so without
   this they silently hide the server.
2. Work out where this session runs. It is a SERVER when the environment variable
   `FACTORY_SERVER` is `1` (set by the server add-on), or when there is no display and no
   browser (`$DISPLAY` and `$WAYLAND_DISPLAY` empty on Linux). Otherwise it is a DESKTOP.
3. For each server that needs an account, guide the user one numbered step at a time, then
   wait for them to say done:
   - DESKTOP, server supports sign-in (OAuth): "Type `/mcp`, choose `<name>`, choose
     Authenticate, finish in the browser, then tell me done."
   - SERVER, or no OAuth: use a token in an environment variable. The `.mcp.json` entry must
     reference it as `${VAR}` (for example `"Authorization": "Bearer ${SUPABASE_ACCESS_TOKEN}"`).
     Tell the user where to create the token (the profile names the page), then give this
     command to run in a terminal on that machine, so the token never passes through chat:
     ```
     mkdir -p ~/.config/claude-rc && read -rsp "Token: " T && printf '<VAR>=%s\n' "$T" >> ~/.config/claude-rc/<project>.env && chmod 600 ~/.config/claude-rc/<project>.env && unset T && echo saved
     ```
     On a server running the add-on service, the service loads that file; tell the user to run
     `systemctl --user restart claude-rc@<project>` afterwards and reopen the session.
     On a desktop, tell them to add `set -a; . ~/.config/claude-rc/<project>.env; set +a` to
     their shell profile, or export the variable before starting `claude`.
   Never ask the user to paste a token into this chat.
4. Verify: run `claude mcp list` with Bash (on a server, prefix
   `set -a; . ~/.config/claude-rc/<project>.env 2>/dev/null; set +a;`). Every project server
   must show Connected. For anything else, show the line and the fix, and offer to retry.

## Finish
1. Delete `.factory/init-state.json`.
2. Show what was written and the MCP status table.
3. Ask whether to commit `.claude/settings.json`, `.mcp.json`, `.factory/stack.md` and
   `.gitignore` as "Set up Pedro's Factory".
4. Tell the user the next command: `/factory:build <feature>`.
