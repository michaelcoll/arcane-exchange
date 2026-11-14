---
name: bump-deps
description: Run `mise run upgrade` in a fresh worktree and open a PR with every file the bumps touched.
disable-model-invocation: true
---

# Bump deps

The whole bump runs in a background `implementer` agent; the main agent dispatches it, stays free, and relays its
report.

## 1. Dispatch

Launch one agent with `subagent_type: "implementer"`, in the background. Its prompt:

> Follow `<skill>/PROCEDURE.md` step by step, where `<skill>` is `<this skill's base directory>`. The main checkout is
> `<repo root>`. You are cleared to commit, push and open the PR. The pre-push hook (`mise run checks`) is your check
> run. End with the report PROCEDURE.md defines, in place of your usual one.

Fill in both absolute paths. Tell the user in one line that the bump is running, then carry on with whatever else they
ask.

## 2. Relay

When the agent's report arrives, act on its **Status**:

- `opened`: give the PR URL and the version list. Offer to remove the worktree and local branch once the PR is merged.
- `current`: say every dependency is up to date.
- `blocked: docker`: ask the user to start Docker, then resume the agent with `SendMessage` from step 5.
- `blocked: push`: hand the push to the user (`git push -u origin HEAD` in the worktree), then resume the agent to open
  the PR.
- `failed`: show the failing command and output; the worktree stays for the user to inspect.
