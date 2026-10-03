# Development and Debugging

How to run the app, prove a change works, and find out why something broke. The container setup itself is described in `README.md` under "Development with Docker".

## Running the app

- `make devc` runs `docker compose up --build dev`. `scripts/dev-desktop.sh` then starts Xvfb (`DISPLAY=:99`, 1600x1000), openbox, x11vnc and noVNC, and runs `pnpm dev:tauri`.
- The checkout is mounted at `/workspace`. Vite hot-reloads the frontend; `tauri dev` rebuilds and restarts the app after Rust changes. Switching branches in the host checkout therefore changes the running app too.
- `http://localhost:1420` is a browser-only preview without Tauri commands. Use noVNC on port 6080 for the real app.
- On the host (macOS), `pnpm dev:tauri` runs a debug build. Debug builds keep their data in `com.skillshare.app-dev`, so they never touch an installed release (`com.skillshare.app`). See `src-tauri/src/utils/paths.rs`.

## Verifying a change

Run the same checks as CI (`.github/workflows/ci.yml`), either on the host or with `docker compose exec dev <command>`:

| Area | Commands |
|---|---|
| Frontend | `pnpm lint`, `pnpm format:check`, `pnpm typecheck`, `pnpm test`, `pnpm build` |
| Rust (in `src-tauri/`) | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` |
| Docs | `python3 scripts/ai-context.py check` |
| Release scripts | `pnpm test:release` |

`cargo clippy` in `src-tauri/` needs `../dist` to exist (`pnpm build` creates it; CI runs `mkdir -p ../dist`).

For UI changes, unit tests are not enough. Look at the running app:

- Open `http://localhost:6080/vnc.html?autoconnect=true&resize=scale` with a browser tool (for example Playwright) and take a screenshot.
- `xdotool` is installed in the container, e.g. `docker compose exec dev xdotool search --name skillshare`. If a click moves the window off-screen, bring it back with `xdotool windowmove <id> 0 0`.
- The container has no native file portal, so "reveal in folder" and similar shell integrations fail there. Check those on macOS.

## Shipping a change

The `skillshare-app-pr` skill walks through these steps.

- Work in a worktree on a new branch from `origin/main`: `git worktree add -b <type>/<name> ../skillshare-app-<name> origin/main`. The main checkout is what the dev container runs, so keep it on a clean `main`. Run `pnpm install --frozen-lockfile` in a new worktree.
- To see a change in the dev container, apply the branch diff to the main checkout temporarily: `git -C <worktree> diff --binary origin/main | git apply --binary`, then `touch src-tauri/src/lib.rs` so `tauri dev` rebuilds. Wait for a new `skillshare-app` PID (`pgrep -x skillshare-app`) before taking screenshots. Afterwards run `git checkout -- . && git clean -fd src src-tauri/src` in the main checkout. The app looks the same as `main` again, so tell the user that before they look at noVNC.
- Commit with Conventional Commits; the body says why. The pre-commit hook runs lint-staged.
- The PR body has two sections: **What** (the behavior change) and **Verification** (the commands run, with counts, and what was seen on screen).
- `scripts/merge-pr.sh <pr>` waits for the checks and reruns a check that failed only because the runner's apt mirror timed out (`Install Linux dependencies … timed out`), up to 3 times. It squash-merges with `--admin` only when every check passes, and otherwise exits without merging. It takes minutes, so run it in the background.
- To update an open PR with `main`, merge `origin/main` into the branch. Never rebase and force-push a pushed branch.
- Two PRs that touch the same files: merge the first, merge `origin/main` into the second, then merge it.
- After the merge, run `git pull --ff-only` in the main checkout and `git worktree remove` the worktree. Merging does not release; wait for the user to ask (`skillshare-app-release`).

## Logs and state

| What | Where |
|---|---|
| App log (Rust `log::` output and the CLI server's output) | `<app data dir>/logs/app.log`; also on stdout of `tauri dev` |
| App data dir | macOS `~/Library/Application Support/com.skillshare.app[-dev]/`; Linux/container `~/.local/share/com.skillshare.app[-dev]/` |
| skillshare CLI operation log | `~/.local/state/skillshare/logs/operations.log` (container: `/root/.local/state/...`) |
| Server pidfile | `<app data dir>/server.pid` |

**About → Export Diagnostics** writes a redacted bundle of these for bug reports.

## Debugging order

1. Reproduce it and read `app.log` first. Backend services prefix their lines (for example `Auto-sync:` and `Source watch:`).
2. If the CLI web UI misbehaves inside the iframe, check whether the same thing happens with `skillshare ui` in a normal browser. If it does, the bug is in the CLI repo (`runkids/skillshare`), not here.
3. If a CLI command works in a terminal but not from the app, suspect the environment the app passes to it, such as the login-shell `PATH` (`src-tauri/src/utils/env.rs`).
4. Write a failing test that reproduces the bug before you fix it.
