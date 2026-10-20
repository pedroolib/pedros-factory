---
name: rework
description: Send a Pedro's Factory job back to its builders with your note.
argument-hint: <job-id> <note>
disable-model-invocation: true
---
Read `${CLAUDE_PLUGIN_ROOT}/skills/build/SKILL.md` for the pipeline. The first word of
`$ARGUMENTS` is the job id; the rest is the user's note.

1. Read the job's `job.json`, `spec.md`, `tickets.md` and the latest reviews.
2. Map the note to the work packages it affects. If that is ambiguous, ask once with
   AskUserQuestion.
3. Reset `round` to 0, set the stage to `build`, and resume (or respawn) the owning
   `factory:builder` agents with the note plus the earlier findings.
4. Continue the pipeline from stage 4 exactly as the build skill describes.
