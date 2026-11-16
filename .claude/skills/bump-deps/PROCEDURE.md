# Bump deps: procedure

Run by the `implementer` agent that `/bump-deps` dispatches. The goal is one **bump PR**: the lockfile and manifest
changes `mise run upgrade` produces (Cargo, pnpm, SwiftPM), off a fresh `origin/main`, in one commit. `<skill>` is
the skill's base directory, given in the prompt.

## 1. Worktree

From the main checkout:

```bash
git fetch origin main
git worktree add -b build/upgrade-deps-$(date +%F) .claude/worktrees/upgrade-deps-$(date +%F) origin/main
```

Work from that worktree for every later step. Run `mise trust` there, then copy the untracked config the hooks and
tests read from the main checkout: `.env`, `frontend/.env`, `ios-app/Config/Local.xcconfig` (those that exist).

Done when `git status` in the worktree is clean on the new branch.

## 2. Upgrade

```bash
mise run upgrade
```

`pnpm update` also installs `node_modules`, so the worktree needs no `mise run setup`.

Done when the task exits 0 and `git status` lists the changed files. Nothing changed: remove the worktree and branch,
report **current**, stop.

## 3. Commit

Stage **every** file `git status` lists: lockfiles, but also the side effects (`frontend/package.json` ranges,
`frontend/.nuxtrc` setup versions, `ios-app/APIClient/Package.resolved`). One commit:

```text
build(deps): upgrade the dependencies
```

Done when `git status` is clean and the branch is one commit ahead of `origin/main`.

## 4. Version list

From the worktree root:

```bash
bash <skill>/scripts/version-list.sh origin/main
```

It prints, per ecosystem, the direct dependencies that moved (`name: old → new`) and a count of the transitive ones.
Use its output as is.

## 5. Push and PR

Check `docker info` first: the pre-push hook needs PostgreSQL. Docker down: report **blocked: docker**, stop.

```bash
git push -u origin HEAD
```

A pre-push failure from a Nuxt test timeout: rerun that check alone; when it passes, report **blocked: push** (the
user pushes by hand). Any other failure is a real break from a bump: report **failed** with the output, PR unopened.

Write the body with the `pr` skill: the Summary is the version list from step 4; Evidence is the pre-push
`mise run checks` passing. Then:

```bash
gh pr create --base main --title "build(deps): upgrade the dependencies" --body-file <body>
```

## Report

Your final message is the report to the main agent:

- **Status**: `opened`, `current`, `blocked: docker`, `blocked: push` or `failed`.
- **PR**: its URL, when opened.
- **Worktree**: its path and branch, or `removed`.
- **Versions**: the step 4 output, verbatim.
- **Failure**: the failing command and its output tail, when blocked or failed.
