---
name: approve
description: Merge a Pedro's Factory job that the approver escalated to you.
argument-hint: <job-id>
disable-model-invocation: true
---
Read `${CLAUDE_PLUGIN_ROOT}/skills/build/SKILL.md` for the pipeline, then for job `$ARGUMENTS`:

1. Read `.factory/jobs/$ARGUMENTS/job.json` and `decision.md`. If the stage isn't `needs-human`,
   say so and stop.
2. Show the user the one-line reason and `git diff --stat main...factory/$ARGUMENTS`, and ask
   once with AskUserQuestion: merge now, or cancel.
3. On merge: do stage 7's APPROVE path (rebase on `main`, rerun the gates, fast-forward `main`).
   If the rebase conflicts or a gate fails, report it and leave the job in `needs-human`.
4. Set the stage to `merged` (or leave `needs-human`) in job.json and report the result.
