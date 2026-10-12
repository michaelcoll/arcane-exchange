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

## TOOLING

- Change files with `Edit` and `Write` (`replace_all` for a mechanical rename), never with a shell rewrite
  (`perl -pi`, `sed -i`, heredoc)
- Search text with `git grep`, read with `Read`
- `LSP` is a deferred tool: load it with `ToolSearch` (`select:LSP`) before the first call. Use it for definition,
  references and hover on Rust and `.ts` files, in particular before changing a public signature. It has no rename
  or diagnostics call, and no server for `.vue`
- The TypeScript server reads `typescript` from the repo root: when `LSP` answers "Could not find a valid TypeScript
  installation", run `ln -s frontend/node_modules node_modules` there (gitignored, one per checkout or worktree)
- Two diagnostics are noise, confirm with the real build before acting on them:
  - SourceKit `No such module` on Swift files: it has no Xcode index. Check with `mise run build-ios`
  - rustc `error communicating with database`: PostgreSQL is down, and the errors stay until rust-analyzer re-checks.
    Check with `mise run lint-clippy`, which runs offline

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

### Triage labels

Les cinq labels canoniques par défaut (needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix). See
`docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/` at the repo root. See [domain.md](docs/agents/domain.md).
