---
name: profile
description: Data/ML profile rules for Pedro's Factory - read by /factory:init and /factory:build, not for direct use.
user-invocable: false
---
# Profile: data (data science and ML)

## INIT
Questions (skip what the repo already answers):
- Environment: uv, pip + venv, conda or poetry; the command that creates it.
- Entry points: the train, evaluate and predict commands (scripts or `python -m ...`).
- A small sample dataset for smoke runs (path or the command that makes one). If none exists,
  the first job should create one.
- Metrics that matter, and the direction that counts as better.
- Data rules: personal data, licenses, anything that must not leave the machine.

Record in stack.md under `## data`: environment command, train/eval/predict commands, sample
data, metrics, data rules, and the gates below.
Always escalate: changes to the train/validation/test split, the evaluation metric code, data
deletion or migration, anything that uploads data or models off the machine.

MCP: none. Tooling check: the environment command works and imports the main libraries.
Hardware: runs adapt to the machine. With an NVIDIA GPU (`nvidia-smi` succeeds), testers may
run a short GPU run; otherwise CPU smoke runs only. The same project works on a laptop with a
GPU and on a CPU-only server.

## BUILD
Builder rules to add to every brief: fixed seeds; no data leakage between splits; notebooks
are for exploration, and logic that matters moves into importable modules with tests;
config through files or flags, not edits to code; never commit data files or model weights.

## GATES
Linting (`ruff check`), type checks if configured, and `pytest`. Notebooks: run them headless
(`jupyter nbconvert --to notebook --execute`) only when the job changed them.

## TESTERS
- `factory-data:data-tester`: pass ONLY ENV_COMMAND, TRAIN_COMMAND, EVAL_COMMAND, SAMPLE_DATA,
  METRICS (with direction), ACCEPTANCE_CRITERIA and REPORT (`round-<n>/data.md`).

## REVIEWERS
- `factory-data:reviewer-data` (REPORT `round-<n>/data-review.md`).

## ESCALATE
Risk tags: `split-change`, `metric-change`, `data-deletion`, `data-egress`.
