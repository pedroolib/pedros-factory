---
name: status
description: Show every Pedro's Factory job in this project with its stage and verdicts.
---
Read every `.factory/jobs/*/job.json`. Print one table, newest first:
`id | feature (short) | stage | round | verdicts | decision`.
Below it, list jobs in `needs-human` with the reason line from their `decision.md` and the
command to answer: `/factory:approve <id>` or `/factory:rework <id> <note>`.
If there are no jobs, say so and suggest `/factory:build <feature>`.
