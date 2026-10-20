---
name: profile
description: Web profile rules for Pedro's Factory - read by /factory:init and /factory:build, not for direct use.
user-invocable: false
---
# Profile: webdev

## INIT
Questions (skip what the repo already answers):
- UI component library: HeroUI v3, shadcn/ui, or none.
- Database/auth: Supabase (ask for the project ref from `supabase.com/dashboard/project/<ref>`),
  another database, or none.
- Figma files for this project? (Figma Dev Mode needs a paid seat; optional.)

Record in stack.md under `## webdev`: UI library, database, styling approach, and
"App must serve on http://localhost:3000 during review".
Always escalate: database migrations, auth and session code, payment code, deploy config.

MCP entries for the project's `.mcp.json` (the agents use exactly these names):
- `ui-kit`, HeroUI: `{"command": "npx", "args": ["-y", "@heroui/react-mcp@latest"]}` (no account)
- `ui-kit`, shadcn: `{"command": "npx", "args": ["-y", "shadcn@latest", "mcp"]}` (no account)
- `db` and `db-readonly`, Supabase (needs an account):
  `{"type": "http", "url": "https://mcp.supabase.com/mcp?project_ref=<ref>"}` and the same URL
  with `&read_only=true`. DESKTOP: sign in with `/mcp` (OAuth). SERVER: add
  `"headers": {"Authorization": "Bearer ${SUPABASE_ACCESS_TOKEN}"}` to both and use a personal
  access token from `supabase.com/dashboard/account/tokens`, variable `SUPABASE_ACCESS_TOKEN`.
Bundled in this plugin (no entry needed): `playwright`; `figma` (DESKTOP: sign in with `/mcp`
when the user has Figma files; otherwise leave it unauthenticated).

Tooling: download the browser the bundled Playwright MCP uses:
`npx -y playwright@$(npm view @playwright/mcp@latest dependencies.playwright) install chromium`
On Linux, if Chromium reports missing libraries, print
`sudo npx -y playwright@<same version> install-deps chromium` for the user to run.

## BUILD
Design checkpoint (only for jobs with user-facing screens), before stage 3:
spawn `factory-webdev:designer` with the job folder. When it returns, show the user the
screenshots in `design/shots/` and its OPEN_QUESTIONS in one AskUserQuestion round. Iterate at
most 2 times.
Builder rules to add to every brief: follow `design/design.md` and `design/tokens.json` exactly;
load the `frontend-design:frontend-design` skill for UI work; take components from `ui-kit`.

## GATES
Start the app with the `Run:` command from stack.md on http://localhost:3000 and wait until it
answers before stage 6. Stop it after the review round.

## TESTERS
- `factory-webdev:ux-tester`: pass ONLY APP_URL (http://localhost:3000/...), PERSONA, GOAL,
  ACCEPTANCE_CRITERIA and REPORT (`round-<n>/ux.md`).

## REVIEWERS
- `factory-webdev:reviewer-ui`: pass APP_URL, the screens to check, the `design/shots/` paths
  and REPORT (`round-<n>/ui.md`). Only for jobs with user-facing screens.

## ESCALATE
Risk tags: `auth`, `payments`, `db-migration`, `rls` (new or changed database tables).
