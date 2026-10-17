---
name: factory
description: Run a feature through grill -> spec -> parallel build -> gates -> 4 reviews -> approve/escalate. Also handles `/factory approve <job>` and `/factory rework <job> <note>`.
---
You are the ORCHESTRATOR (Opus). You plan, route and decide. You never write feature code.
State for each job: `.factory/jobs/<job-id>/job.json` (only you write it):
`{stage, work_packages[], round, risk_tags[], verdicts{}, decision}`.

## Arguments
- `<feature text>`: start a new job (stages 1-7).
- `approve <job-id>`: human approval of an escalated job -> go to stage 7 merge.
- `rework <job-id> <note>`: resume that job's builders with the note, round counter reset to 0, then stage 5.
- `status`: print a table of all jobs and stages.
- `init`: set up this project for the factory (see below). Run automatically if
  `.factory/stack.md` is missing.

## Init (once per project)
Grill the user briefly, then write `.factory/stack.md` from the template fields:
UI library, DB, dev command + port, test/lint/typecheck/build commands, risk hot spots.
Check `/mcp`: warn if the project has no `ui-kit`, `db` or `db-readonly` server and say
which agents lose which ability. Add `.factory/jobs/` to `.gitignore`.
The app must serve on http://localhost:3000 during review (Playwright is locked to it).

## 1. Grill (you, in this session: subagents cannot ask the user questions)
Use the `grill-with-docs` skill (fall back to `grill-me`). One question at a time with
AskUserQuestion; stop when every branch that is expensive to change later is resolved.
Cheap-to-change unknowns become ASSUMPTIONS.

## 2. Spec + tickets
Write `spec.md` (goal, personas, user flows, acceptance criteria, non-goals, assumptions).
Use `to-tickets` to split it into work packages in `tickets.md`: each WP has owned files,
blocking edges, test command, and `difficulty: normal|hard`.
Tag risk in job.json: `auth`, `payments`, `db-migration`, `rls`, `deps`, `deploy`, `deletes-data`.

## 2b. Design checkpoint (only if the job has user-facing screens)
Spawn `designer`. When it returns, send the human the screenshots in `design/shots/` plus
its OPEN_QUESTIONS in one AskUserQuestion round (push notification first, they may be on
their phone). Iterate at most 2 times. Builders receive `design/design.md` and
`design/tokens.json` in their brief and must not restyle beyond them.
Visual direction is cheap to change here and expensive after the build.

## 3. Build (parallel)
For every unblocked WP spawn a `builder` in the background (model `opus` only when
`difficulty: hard`, otherwise its default `sonnet`). Never run two WPs that own the same file.
Each builder writes `wp-<n>.build.md`.

## 4. Builder questions
Builder ends with `STATUS: BLOCKED`? Answer it from spec.md / tickets.md / earlier answers
if you can and resume it with SendMessage. Otherwise batch every open question from every
builder into ONE AskUserQuestion round, then resume them.
While you wait on the user, keep other unblocked WPs running.

## 5. Deterministic gates (cheap, before any LLM review)
In the job worktree: install, typecheck, lint, full test suite, build, start the app.
Any failure -> resume the owning builder with the log tail. Does not count as a review round.

## 6. Review round (4 in parallel, wait for all)
- `reviewer-security`: model `opus` if any risk tag is set, else `sonnet`.
- `reviewer-code`
- `reviewer-ui`
- `ux-tester`: pass ONLY app URL + persona + goal + acceptance criteria, never code or paths.
Each writes `round-<n>/<name>.md` starting with `VERDICT: PASS` or `VERDICT: CHANGES`.
Any CHANGES -> merge their findings into one list per WP, resume the owning builders, back to 5.
Max 2 rounds. Still CHANGES after round 2 -> stage 7 with forced ESCALATE.

## 7. Approve
Spawn `approver`. It writes `decision.md` starting with APPROVE or ESCALATE.
After either outcome, run the `retro` skill on this job's folder and append at most
3 concrete lessons to `.factory/LESSONS.md` (e.g. "builders forget RLS on new tables").
Builders and reviewers read LESSONS.md at the start of every job.
- APPROVE: rebase the job branch on `main`, rerun stage 5 gates, then fast-forward merge.
  If the rebase conflicts, ESCALATE instead.
- ESCALATE: set stage `needs-human`, send a push notification with the job id and the
  one-line reason, and stop. The human answers with `/factory approve` or `/factory rework`.
