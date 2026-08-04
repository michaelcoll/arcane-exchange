# CRITICAL RULES - MUST FOLLOW

## RESPONSES

- Keep responses concise and to the point – unless the user asks otherwise
- Respond to the user using his language

## PLANNING MODE

- Always ask clarifying questions
- Never assume design, tech stack or features

## DESTRUCTIVE ACTIONS

- Before any destructive or hard-to-reverse action, stop and ask for explicit confirmation first — never assume consent
  from a prior instruction on a different task
- This includes (non-exhaustive): dropping/truncating DB tables or schemas, running migrations that drop columns or
  data, `rm -rf`, `git reset --hard`, `git push --force`, `git clean`, deleting branches, overwriting uncommitted
  changes, and any `mise run` task whose effect is destructive (e.g. `clean`, `migrate` if it involves down-migrations)
- State plainly what will be destroyed (table, file, branch, data) and wait for a clear yes before running it — a vague
  or implied approval is not enough

## CHANGE / EDIT MODE

- After completing features (large or small), always run `mise run lint-backend` and/or `mise run lint-frontend` to type
  check, and `mise run format` to format the code
- Always use `mise run <task>` (see [mise.instructions.md](.agents/mise.instructions.md)) instead of calling
  `cargo`/`pnpm`/`npm` directly — if a mise task exists for what you're doing, use it rather than the raw command

## TESTING

- Use any testing tools, libraries available to the project for testing your changes
- Never assume your changes simply work, always test!

## PARALLELIZATION

- Always launch independent tool calls in parallel within the same message
- Never wait for one tool's result before calling another tool that doesn't depend on it

## TOOLS

- Reading a file → `Read`, never `cat`/`head`/`tail`/`sed`/piping through `python`/`jq` in Bash
- Searching file content → `Grep`, never `grep`/`ack`/`rg` in Bash
- Finding files by name/pattern → `Glob`, never `find` in Bash
- Editing a file → `Edit` (diff-only), never `sed`/`awk`
- Creating a file → `Write`, never heredoc/`echo >` in Bash
- Reserve `Bash` for what only a shell can do: running `mise run <task>`, git, moving/deleting files
- If a dedicated tool exists for the job, using Bash instead is a mistake — not a style choice
- Prefer `mise run <task>` over calling `cargo`/`pnpm` directly — see
  [mise.instructions.md](.agents/mise.instructions.md) for the full task list

## PLAYWRIGHT

- Save your screen captures and logs from playwright in the `.playwright-mcp` folder at the root of the repository

## Instructions

- **Authentication**: [authentication.instructions.md](.agents/authentication.instructions.md)
- **Backend**: [backend.instructions.md](.agents/backend.instructions.md)
- **CI/CD**: [ci.instructions.md](.agents/ci.instructions.md)
- **Database Schema**: [database-schema.instructions.md](.agents/database-schema.instructions.md)
- **Design System**: [design-system.instructions.md](.agents/design-system.instructions.md)
- **API Endpoints**: [endpoints.instructions.md](.agents/endpoints.instructions.md)
- **Frontend**: [frontend.instructions.md](.agents/frontend.instructions.md)
- **Mise & Workflow**: [mise.instructions.md](.agents/mise.instructions.md)
- **Trade Workflow**: [trade-workflow.instructions.md](.agents/trade-workflow.instructions.md)
