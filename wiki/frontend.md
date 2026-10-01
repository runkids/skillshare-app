# Frontend

React 19 + TypeScript + Vite 8 + Tailwind CSS 4, rendered in a Tauri 2 webview. The app is a thin shell: the real dashboard is the skillshare CLI's own web UI, loaded in an iframe.

## Layout

- `src/main.tsx` mounts `App` in `StrictMode` and imports `src/index.css`.
- `src/App.tsx` nests providers in this order: `QueryClientProvider` > `ThemeProvider` > `TauriProvider` > `ProjectProvider` > `TerminalProvider` > `BrowserRouter` > `ErrorBoundary`. It renders `UpdateCheckListener`, then an `OnboardingGuard` around four routes: `/onboarding`, `/settings`, `/activity`, and `/*` (`MainView`).
- `src/desktop/api/tauri-bridge.ts`: every Rust `invoke` call plus the shared types (`AppInfo`, `Project`, `AvailableUpdates`, ...).
- `src/desktop/pages/`: `OnboardingPage`, `SettingsPage`, `ActivityPage` (`/activity`, opened from the title bar; day grouping and relative times in `utils/activity.ts`).
- `src/desktop/components/`: `MainView`, `TitleBar`, `SourceHealthBadge`, `AuditBadge`, `ProjectDropdown`, `CliWebView` (iframe host), `UpdateCheckListener`, `ResourceUpdatesBadge`, `TerminalAccess`; subfolders `OnboardingSteps/` (steps, a pure `onboarding-flow.ts` reducer, `onboarding.css` with `ob-*` classes), `settings/` (one component per tab), `terminal/` (xterm UI, currently hidden by `SHOW_TERMINAL = false` in `MainView`).
- `src/desktop/context/`: `TauriContext` (`useTauri`: `appInfo`, `loading`, `refresh`), `ProjectContext` (`useProjects`: projects, `activeProject`, `switching`, `switchWithRestart`, `reloadKey`/`reloadView`), `TerminalContext` (`useTerminal`).
- `src/desktop/hooks/`: `useUpdates`, `useSourceHealth`, `useAudit` and `useAppUpdate` (module-level stores read with `useSyncExternalStore`), `useCliManager`, plus terminal hooks `usePtySpawn`, `useTerminalInstance`.
- `src/desktop/utils/`: `platform.ts` (`detectPlatform`, `isMacOS` from the user agent), `path.ts` (`shortPath`).
- `src/components/`: shared primitives `Button`, `Card`, `Input`, `Switch`, `Badge`, `Spinner` (default exports) and `ErrorBoundary` (named export).
- `src/context/`: `ThemeContext.tsx` (provider) and `useTheme.ts` (context, types, `useTheme`).
- `src/lib/queryClient.ts`: the TanStack Query client. No component calls `useQuery` yet.
- `src/design.ts`: a `shadows` map of `var(--shadow-*)` strings for inline styles; only `Card` uses it.
- `src/theme-tokens.css`: the CSS custom properties for every theme (see Styling).

## App flow

1. `TauriProvider` calls `getAppState` once. `OnboardingGuard` renders nothing while loading and redirects to `/onboarding` when `appInfo.onboarding.completed` is false. If `appInfo` is null (no Tauri) it lets every route through.
2. `OnboardingPage` drives three steps (`WelcomeStep` finds or installs the CLI, `ProjectSetupStep`, `FirstSyncStep`) through `flowReducer`. On finish it calls `startServer(cliPath)`, refreshes app and project state, then navigates to `/`.
3. `MainView` shows `TitleBar` and `CliWebView`. `CliWebView` uses `http://localhost:${appInfo.serverPort}` when the server already runs; otherwise it calls `detectCli` then `startServer(cliPath, activeProject.path)` once and uses the returned port. The iframe `src` is that origin plus the shell path (a shell route like `/skills?tab=updates` opens that CLI page) and a `?theme=` param.
4. `CliWebView` polls `healthCheck` every 30 s; three failures show the "server-down" card with Restart. A `reloadKey` change (title bar reload, `sync-completed`) reloads the iframe, or restarts the server when unhealthy. Finishing a project switch remounts the iframe.
5. `TitleBar` shows one resource update count (skills, repositories, agents, plugins); clicking it lists each kind and links to the CLI's corresponding review page. App/CLI versions keep their settings indicator.
6. `/settings?tab=general|appearance|projects|cli|about` picks the tab via `useSearchParams`; the nav shows an "Update" badge from `useUpdates`/`useAppUpdate`.

Rust events (names verified in `src-tauri/src`), all subscribed with `listen` from `@tauri-apps/api/event` inside a `useEffect` guarded by `isTauri()`, and unsubscribed with `unlisten.then((off) => off())`:

- `server-restarted` (payload: port) and `server-stopped`: `CliWebView`.
- `updates-available` (payload: `AvailableUpdates`): `useUpdates`.
- `source-health` (`SOURCE_HEALTH_EVENT`, payload: `SourceHealth`): `useSourceHealth`, shown by `SourceHealthBadge` in `TitleBar`, which opens the Web UI `/sync` (target drift) or `/git`.
- `audit-report` (`AUDIT_EVENT`, payload: `AuditFinding[]`): `useAudit`, shown by `AuditBadge` in `TitleBar`; its list links each CRITICAL/HIGH/MEDIUM finding to the Web UI `/skills/<name>` or `/agents/<name>`, counts LOW ones, and opens `/audit`.
- `check-for-updates` (menu/tray): `UpdateCheckListener` opens `/settings?tab=about` and rechecks.
- `sync-completed` (`SYNC_COMPLETED_EVENT`, tray or auto Quick Sync): `ProjectContext` calls `reloadView`; `ActivityPage` re-reads the log.
- `cli-install-output` (`CLI_INSTALL_OUTPUT_EVENT`): `useCliManager`.

App self-update uses `check()` from `@tauri-apps/plugin-updater` in `useAppUpdate`, not the bridge.

## Talking to Rust

All commands go through the `tauriBridge` object in `src/desktop/api/tauri-bridge.ts`, one arrow function per command wrapping `invoke<T>('snake_case_name', { camelCaseArgs })`. To add a call:

1. Add the `#[tauri::command]` in `src-tauri/src/commands/*.rs` and register it in `generate_handler!` in `src-tauri/src/lib.rs`.
2. Add a typed method to `tauriBridge` (and any result interface next to it). Tauri maps camelCase arg keys to the Rust snake_case parameters.
3. Call it from components as `tauriBridge.foo()`; do not call `invoke` directly elsewhere.

Plugins (`plugin-dialog`, `plugin-opener`, `plugin-process`, `plugin-updater`, `api/app`) are imported directly where used.

Browser preview: `pnpm dev` serves Vite on port 1420 (`strictPort`) and proxies `/api` (plus SSE stream paths) to `http://localhost:19420`. Without the Tauri runtime, `getAppState` rejects, `appInfo` stays null, and `CliWebView` renders a "Browser preview" placeholder instead of starting the CLI. Any new native call or `listen` should be guarded with `isTauri()` from `@tauri-apps/api/core` so the preview keeps working (`browser-preview.test.tsx` checks this).

## Styling and theme

- Tailwind 4 via `@tailwindcss/vite`; no `tailwind.config`. `src/index.css` imports `tailwindcss` and `theme-tokens.css`, declares `@custom-variant dark` on the `.dark` class, and maps tokens to utilities in `@theme inline` (`bg-bg`, `bg-surface`, `text-ink`, `border-line`, `text-ink-2`, ...). Legacy aliases (`paper`, `pencil`, `muted`, `accent`, `danger`, ...) map onto the same tokens.
- Components mostly use arbitrary values on raw tokens, e.g. `bg-[var(--surface)]`, `rounded-[var(--r-box)]`, `shadow-[var(--sh-box)]`, `border-[length:var(--bw)]`. Prefer tokens over hex colors.
- `theme-tokens.css` defines four token sets: Clean light, Clean dark, Playful light, Playful dark. Style is `html[data-theme="playful"]` (absent means Clean); mode is the `dark` class on `html`.
- `ThemeProvider` keeps two axes: `style` (`clean|playful`, localStorage `skillshare-style`) and `modePreference` (`light|dark|system`, localStorage `skillshare-theme-preference`; migrates the legacy `skillshare-theme` key). `system` follows `prefers-color-scheme`. Changes briefly add `theme-transitioning` to `html`.
- Iframe sync in `CliWebView`: the initial URL carries `?theme=` (`playful`, `dark` or `clean`). After load, and on every change, it posts `{ type: 'theme-push', mode, style }` to the iframe. It accepts `{ type: 'theme-change', theme }` messages only from the current loaded iframe; `playful`/`clean` set style, `dark`/`light` set mode.

## Tests

- Vitest with jsdom, configured in `vite.config.ts`; `src/test/setup.ts` loads `@testing-library/jest-dom/vitest` and runs `cleanup` after each test. Run `pnpm test`.
- Tests are co-located: `*.test.tsx` / `*.test.ts` next to the code.
- Mock at module level with `vi.mock`: `../api/tauri-bridge` (return `{ tauriBridge: { fn: vi.fn() } }`), the contexts (`useTauri`, `useProjects`, `useTerminal`), `@tauri-apps/api/core` as `{ isTauri: () => true }`, and plugins. No test uses `mockIPC`.
- For events, mock `listen` to store handlers in a `vi.hoisted` map and fire them inside `act`. Wrap routed components in `MemoryRouter`; stub `matchMedia` with `vi.stubGlobal` when `ThemeProvider` or theme code runs.
- Example to copy: `src/desktop/components/server-events.test.tsx`. Module-level stores expose reset helpers (e.g. `resetAppUpdateForTests` in `useAppUpdate`).

## Conventions

- Package manager is pnpm (`pnpm-lock.yaml`). Scripts: `lint` (`eslint .`), `typecheck` (`tsc -b`), `test`, `format` / `format:check` (Prettier on `src/**/*.{ts,tsx}`).
- ESLint flat config (`eslint.config.js`): `@eslint/js` recommended, `typescript-eslint` recommended, `react-hooks` recommended, `react-refresh` vite. A hook exported beside its provider needs `// eslint-disable-next-line react-refresh/only-export-components`. `exhaustive-deps` disables carry a reason after `--`.
- Prettier (`.prettierrc`): single quotes, semicolons, 2-space indent, `printWidth` 100, trailing commas `es5`, LF. Husky pre-commit runs `lint-staged` (`eslint --fix` + `prettier --write`).
- TypeScript is `strict` with `noUnusedLocals`, `noUnusedParameters`, `verbatimModuleSyntax` (use `import type`) and `erasableSyntaxOnly` (no enums or parameter properties).
- No import aliases: all imports are relative (`../../components/Button`). Imports may include `.tsx` extensions (`allowImportingTsExtensions`).
- Components are default-exported function components; contexts and hooks use named exports. Icons come from `lucide-react`.
