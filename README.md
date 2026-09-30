<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" alt="Skillshare App" width="120" height="120">
</p>

<h1 align="center">Skillshare App</h1>

<p align="center">
  Desktop app for <a href="https://github.com/runkids/skillshare">skillshare CLI</a>.
</p>

<p align="center">
  <a href="https://github.com/runkids/skillshare-app/releases">
    <img src="https://img.shields.io/github/v/release/runkids/skillshare-app?style=for-the-badge&color=blue" alt="Release">
  </a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey?style=for-the-badge" alt="Platform">
</p>

---

## Prerequisites

Install [skillshare CLI](https://github.com/runkids/skillshare) first.

## Install

### Homebrew (macOS)

```bash
brew tap runkids/tap
brew install --cask skillshare-app

# Uninstall
brew uninstall --cask skillshare-app
```

### Direct Download

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [.dmg](https://github.com/runkids/skillshare-app/releases/latest) |
| Windows | [.exe](https://github.com/runkids/skillshare-app/releases/latest) / [.msi](https://github.com/runkids/skillshare-app/releases/latest) |
| Linux | [.deb](https://github.com/runkids/skillshare-app/releases/latest) / [.AppImage](https://github.com/runkids/skillshare-app/releases/latest) / [.rpm](https://github.com/runkids/skillshare-app/releases/latest) |

Or browse all releases: **[GitHub Releases](https://github.com/runkids/skillshare-app/releases)**

## Development with Docker

The host needs Docker and Docker Compose, plus `make` for the shortcut below.
Node.js, pnpm, Rust, mise, and Linux build dependencies are installed inside the
development image. Tool versions are pinned in `mise.toml`; the mise version is
pinned in `Dockerfile.dev`.

Start the Linux Tauri app in development mode:

```bash
make devc
```

`make devc` runs `docker compose up --build dev`. The first run installs dependencies
and compiles Rust. Once compilation finishes, open
<http://localhost:6080/vnc.html?autoconnect=true&resize=scale> to use the full desktop
app inside the container through noVNC. Rust, Node.js, and Linux GUI tools do not
need to be installed on the host.

Source code is mounted from the local checkout. Vite provides frontend hot updates,
and `tauri dev` recompiles and restarts the app after Rust changes. This runs the
Linux desktop app, with native Tauri commands and the backend inside the container.
It does not open a native macOS window. <http://localhost:1420> provides a limited
browser frontend preview; use the remote desktop on port `6080` for full functionality.

On first launch, follow onboarding to download the skillshare CLI, initialize its
configuration, and sync. The CLI is also installed inside the container. App settings,
the CLI, skills, and onboarding state persist in the `app_data` and `app_config`
volumes. JavaScript dependencies and Rust caches have separate persistent volumes.
The checkout is mounted at `/workspace`; the app does not read the host's skillshare
configuration.

Closing the app window hides it and leaves the backend running. Use the tray menu's
**Open Skillshare App** action to restore the window. On macOS, clicking the Dock
icon also restores it. The remote desktop port is bound to the host's `127.0.0.1`
interface.

With the development container running, run checks from another terminal:

```bash
docker compose exec dev pnpm lint
docker compose exec dev pnpm build
docker compose exec dev pnpm test
docker compose exec dev cargo check --locked --manifest-path src-tauri/Cargo.toml
docker compose exec dev cargo test --locked --manifest-path src-tauri/Cargo.toml
```

Build the Linux desktop executable without opening a desktop window:

```bash
docker compose exec dev pnpm tauri build --no-bundle
```

The Linux executable and Cargo build artifacts are stored in the `cargo_target`
volume. Building a macOS app requires a separate macOS build environment, such as
the existing release CI.

Open a shell with the mise-managed tools:

```bash
docker compose run --rm dev bash
```

After changing `mise.toml`, rebuild the image with `docker compose up --build dev`.
Press `Ctrl+C` to stop the development container, then run `docker compose down`
to remove the stopped container. Named volumes are preserved for the next run.

## License

MIT
