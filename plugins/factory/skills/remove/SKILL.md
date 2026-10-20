---
name: remove
description: Remove Pedro's Factory from this project, or with --everything uninstall it from this machine.
argument-hint: [--everything]
disable-model-invocation: true
---
Never touch the user's own code. Before deleting or editing anything, list exactly what will go
and ask ONE AskUserQuestion to confirm. Show diffs for files you edit instead of deleting them.

## This project (no argument)

Collect, then confirm in one question (with a checkbox for each optional item):
1. **Plugins**: every `enabledPlugins` key in `.claude/settings.json` ending in `@pedros-factory`.
   Remove each with `claude plugin uninstall <plugin> --scope project`. This only removes them
   from this project; machine-wide installs stay.
2. **Config written by init**:
   - `.factory/stack.md` and `.factory/init-state.json`
   - in `.claude/settings.json`: the `model` and the two `env` keys init added
     (`CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH`, `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS`)
   - in `.claude/settings.local.json`: the factory server names in `enabledMcpjsonServers`
   - the `.gitignore` lines init added (`.factory/jobs/`, `.factory/init-state.json`)
3. **Optional, ask**: the MCP servers init wrote to `.mcp.json` (`ui-kit`, `db`, `db-readonly`);
   the app may still use them, so default to keeping them.
4. **Optional, ask**: job history (`.factory/jobs/`, `.factory/LESSONS.md`). Default: keep.
5. **Optional, ask**: the secrets file `~/.config/claude-rc/<project>.env`. Tokens there can't be
   recovered, so default to keeping it.
6. **Server only**: if `systemctl --user is-enabled claude-rc@<project>` succeeds, don't stop it
   yourself (it may be running this very session). Print
   `systemctl --user disable --now claude-rc@<project>` for the user to run.

Delete empty `.factory/` and `.claude/` folders and empty JSON files you emptied. Finish with
what was removed, what was kept, and an offer to commit as "Remove Pedro's Factory".
Tell the user to run `/reload-plugins`.

## Whole machine (`--everything`)

Confirm in one question, then:
1. `claude plugin marketplace remove pedros-factory`. This uninstalls every plugin from it on
   this machine, including project installs. Projects keep their `.factory/` files and
   `.claude/settings.json` entries: say so, and that `/factory:remove` in a project (before this
   step) is the way to clean one.
2. **Optional, ask**: rtk. Remove the `rtk hook claude` entry from `~/.claude/settings.json`, the
   `@RTK.md` line from `~/.claude/CLAUDE.md`, `~/.claude/RTK.md` and `~/.local/bin/rtk`.
   Default: keep (it's useful without the factory).
3. **Optional, ask**: `remoteControlAtStartup` in `~/.claude/settings.json`, if setup set it.
4. **Server only**: if `~/.config/systemd/user/claude-rc@.service` exists, print the commands
   to disable every `claude-rc@*` service and remove the unit, for the user to run.

Finish with what was removed and kept, and tell the user to restart Claude Code.
