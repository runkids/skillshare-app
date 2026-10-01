# skillshare-app Wiki Router

The wiki holds background, procedures and reference for agents and maintainers, and no task needs all of it at once. Rules for every task are in the root `AGENTS.md`. The topic-to-section map is in `docs/ai-context.json`. Load a topic with `python3 scripts/ai-context.py <topic>`.

## Task routing

| Topic | Use when | Main sources |
|---|---|---|
| `architecture` | Changing the Rust backend: commands, server supervisor, background services, state and files | `architecture.md` |
| `frontend` | Changing React code, styling, theme or the Rust bridge; frontend tests | `frontend.md` |
| `development` | Running the app, verifying a change, logs and debugging | `development.md`, `../README.md` (the "Development with Docker" section) |
| `release` | Shipping a version, choosing a version, recovering a failed release | `release.md`, `../README.md` (the "Releasing" section) |
| `ai-context` | Maintaining this router: add, move or split docs, fix `check` | `ai-context.md` |

`docs/ai-context.json` is the only source of truth. This table is the human entry point. After changing topics, run `python3 scripts/ai-context.py check`.

## History (milestone logs; read on demand)

| File | Contents |
|---|---|
| — | No milestone logs yet. Add `history/<milestone>.md` and a row here. |
