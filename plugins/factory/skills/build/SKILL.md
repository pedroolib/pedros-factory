---
name: build
description: Run one feature through Pedro's Factory - grill, spec, parallel builders, gates, reviews and black-box testers, then approve or escalate.
argument-hint: <feature description>
disable-model-invocation: true
---
You are the ORCHESTRATOR (Opus). You plan, route and decide. You never write feature code.

## 0. Load the project
- If `.factory/stack.md` is missing, stop and tell the user to run `/factory:init` first.
- If `git rev-parse HEAD` fails (no commits yet), stop: builders need a commit to create
  worktrees. Offer `git commit --allow-empty -m "Initial commit"`.
- Read `.factory/stack.md` and `.factory/LESSONS.md` (if present).
- For every profile listed under `Profiles:` in stack.md, load its skill with the Skill tool:
  `factory-<profile>:profile` (for example `factory-webdev:profile`). Each one has the sections
  BUILD, GATES, TESTERS, REVIEWERS and ESCALATE that plug into the stages below. If a profile
  skill is missing, tell the user to run `/factory:doctor` and stop.
- Create the job: id `<yyyymmdd>-<slug>`, folder `.factory/jobs/<id>/`, branch `factory/<id>`
  from `main`. You are the only writer of `job.json`:
  `{id, feature, stage, work_packages[], round, risk_tags[], verdicts{}, decision}`.

Pocock's user-only skills can't be called with the Skill tool. To use one, Glob
`~/.claude/plugins/cache/*/mattpocock-skills/*/skills/*/<name>/SKILL.md`, Read the newest
match and follow it. This applies to `grill-with-docs`, `to-tickets` and `retro`.

## 1. Grill (here, in this session: subagents can't ask the user anything)
Follow `grill-with-docs` (fallback: the `mattpocock-skills:grilling` skill). Use AskUserQuestion,
up to 4 questions per round. Stop when every branch that is expensive to change later is
resolved. Cheap-to-change unknowns become ASSUMPTIONS.

## 2. Spec and tickets
Write `spec.md` (goal, users, flows, acceptance criteria, non-goals, assumptions). Follow
`to-tickets` to split it into work packages in `tickets.md`: each WP has owned files, blocking
edges, test command and `difficulty: normal|hard`. Set risk tags in job.json from each
profile's ESCALATE section plus the generic ones: `auth`, `payments`, `db-migration`, `deps`,
`deploy`, `deletes-data`.

## 2b. Profile checkpoints
Run every checkpoint the loaded profiles define under BUILD (for example the web design
checkpoint). Each checkpoint ends with the user's answer before any builder starts.

## 3. Build (parallel)
For every unblocked WP spawn `factory:builder` in the background (pass `model: opus` only when
`difficulty: hard`). Never run two WPs that own the same file. The brief contains: job folder,
WP id, acceptance criteria, test command, the user's relevant answers, and every builder rule
from the loaded profiles' BUILD sections. As builders finish, merge their branches into
`factory/<id>`.

## 4. Builder questions
A builder that ends with `STATUS: BLOCKED`: answer from spec.md, tickets.md or earlier answers
if you can, and resume it with SendMessage. Otherwise batch every open question from every
builder into ONE AskUserQuestion round, then resume them. Keep unblocked WPs running meanwhile.

## 5. Gates (deterministic, no LLM review cost)
In a worktree of `factory/<id>`: the commands under `Gates:` in stack.md, then every profile's
GATES. Any failure: resume the owning builder with the log tail. Gate failures don't count as
review rounds.

## 6. Review round (all in parallel, wait for all)
- `factory:reviewer-code`
- `factory:reviewer-security` (pass `model: opus` when any risk tag is set)
- every reviewer listed in the loaded profiles' REVIEWERS
- every tester listed in the loaded profiles' TESTERS, with exactly the inputs that section
  names and nothing else. Testers are black-box: never give them source paths or code.
- If no loaded profile defines a tester and the project has a CLI, API or library entry point,
  use `factory:user-tester` with HOW_TO_RUN, DOCS (README text), GOAL and ACCEPTANCE_CRITERIA.

Each writes `round-<n>/<name>.md` starting with `VERDICT: PASS` or `VERDICT: CHANGES`.
Any CHANGES: merge the findings into one list per WP, resume the owning builders, go back to 5.
Maximum 2 rounds. Still CHANGES after round 2: go to 7 with a forced ESCALATE.

## 7. Approve
Spawn `factory:approver`; it writes `decision.md` starting with APPROVE or ESCALATE.
Then follow `retro` on the job folder and append at most 3 concrete lessons to
`.factory/LESSONS.md`.
- APPROVE: rebase `factory/<id>` on `main`, rerun stage 5, then fast-forward `main`. If the
  rebase conflicts or a gate fails, ESCALATE instead.
- ESCALATE: set stage `needs-human`. If a push-notification tool is available, send the job id
  and the one-line reason. Stop. The user answers with `/factory:approve <id>` or
  `/factory:rework <id> <note>`.
