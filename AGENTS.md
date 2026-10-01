# AGENTS.md — skillshare-app

skillshare-app is a Tauri 2 desktop app for the [skillshare](https://github.com/runkids/skillshare) Go CLI. The Rust backend (`src-tauri/`) installs and updates the CLI, runs `skillshare ui` as a supervised local server, and adds native features: tray, notifications, project list, terminal, auto-sync and update checks. The React frontend (`src/`) handles onboarding and settings and shows the CLI's web UI in an iframe. CLI behavior belongs in the CLI repo, not here.

## Language

- Code, identifiers, comments, commit messages and instruction files: English.
- Reply to users in their language; default to Traditional Chinese (Taiwan) when they write Chinese.

## Layout

| Path | What lives there |
|---|---|
| `src-tauri/src/commands/` | Tauri IPC commands, registered in `lib.rs` |
| `src-tauri/src/services/` | CLI install/update (`cli_manager`), server supervisor (`server_manager`), `auto_sync`, `update_watch`, `diagnostics` |
| `src-tauri/src/main_window.rs` | Main window, built in code (`tauri.conf.json` sets `"create": false`); link, navigation and download handling |
| `src/` | React + TypeScript frontend, with tests next to the code (Vitest) |
| `.github/workflows/` | `ci.yml` checks; Release Please, `release.yml` and `update-homebrew.yml` release the app |
| `.skillshare/skills/` | Project skills (source of truth) |

## Rules

- **Verify with evidence.** Check claims against current source, installed versions, or command output before acting on them.
- **Keep scope minimal.** Inspect the working tree first, never overwrite unrelated user changes, and match the surrounding style.
- **Keep docs true.** When a change alters behavior that `README.md` or a skill describes, update it in the same change.
- **Use Git safely.** Conventional Commits in English, with a body that explains why. Never use `--no-verify`, never amend or force-push pushed commits, and do not commit, merge, tag or release without the user's authorization.
- **Never commit secrets.** The signing, notarization and Homebrew credentials live only in GitHub Actions secrets.

## Verification

CI (`.github/workflows/ci.yml`) runs these, and all must pass:

```sh
pnpm lint && pnpm format:check && pnpm typecheck && pnpm test && pnpm build
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

The pre-commit hook runs lint-staged (`eslint --fix`, `prettier --write`) on staged `.ts`/`.tsx` files.

To run the Linux desktop app without installing a toolchain on the host, use `make devc`, then open <http://localhost:6080/vnc.html?autoconnect=true&resize=scale>. Run checks with `docker compose exec dev <command>`. For UI changes, take a screenshot of the running app; unit tests alone are not enough.

## Skills

Project skills are managed by skillshare. Edit them in `.skillshare/skills/`, then run `skillshare sync -p` to link them into `.claude/skills/` and `.agents/skills/`. Those directories are generated, so don't edit them.

| Skill | Use when |
|---|---|
| `skillshare-app-release` | Shipping a version: the Release PR, choosing a version, the build, and checks after publishing. Release Please owns version numbers, so never bump them by hand. |
