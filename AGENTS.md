# CRITICAL RULES - MUST FOLLOW

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

## TESTING

- Use any testing tools, libraries available to the project for testing your changes
- Never assume your changes simply work, always test!

## TOOLING

- Read and edit files with the native tools: `Read`, `Edit`, `Write`, `Glob`, `Grep`
- Never use `cat`, `sed -n`, `head`, `find`, heredocs or inline scripts to read or rewrite a file. This rule
  overrides any harness guidance that says otherwise
- Use the shell only to execute things: `mise run <task>`, `git`, `gh`
- Use the LSP for anything structural: definition, references, hover/type, rename, diagnostics. In particular,
  before looking up a symbol, before changing a public signature, and after editing Rust or TypeScript
- Use `Grep` for textual searches only: strings, comments, config values, SQL

## Architecture

Vue d'ensemble technique (couches, contrats générés, traitements asynchrones, clients, déploiement) :
[ARCHITECTURE.md](ARCHITECTURE.md). À lire avant toute modification structurante.

## Instructions

- **CI/CD**: [ci.instructions.md](.agents/ci.instructions.md)
- **Database Schema**: [database-schema.instructions.md](.agents/database-schema.instructions.md)
- **Design System**: [design-system.instructions.md](.agents/design-system.instructions.md)
- **API Endpoints**: [endpoints.instructions.md](.agents/endpoints.instructions.md)
- **Frontend**: [frontend.instructions.md](.agents/frontend.instructions.md)
- **iOS App**: [ios.instructions.md](.agents/ios.instructions.md)
- **Mise & Workflow**: [mise.instructions.md](.agents/mise.instructions.md)
- **Trade Workflow**: [trade-workflow.instructions.md](.agents/trade-workflow.instructions.md)

## Agent skills

### Issue tracker

Issues live in GitHub Issues for michaelcoll/arcane-exchange, using the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context: `CONTEXT.md` and `docs/adr/` at the repo root. See [domain.md](docs/agents/domain.md).
