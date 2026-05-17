# AGENTS.md

Read `STATUS.md` first.

## Project Rules

- Keep the compatibility runner separate from `cdk/` and `nutshell/`.
- Do not modify upstream cloned repos unless explicitly required.
- Update `STATUS.md` whenever progress, decisions, or blockers change.
- Prefer the smallest useful change.

## Current Direction

- Build a standalone compatibility tool in this workspace.
- First target a local zero-fee CDK mint.
- Produce both terminal table output and JSON output.
- First milestone: 2-3 simple spending-condition tests working against CDK mint.
