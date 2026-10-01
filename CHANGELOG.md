# Changelog

## [0.0.14](https://github.com/runkids/skillshare-app/compare/v0.0.13...v0.0.14) (2026-10-01)


### Features

* **about:** link the app repo and release notes, and export diagnostics ([#22](https://github.com/runkids/skillshare-app/issues/22)) ([a105b4e](https://github.com/runkids/skillshare-app/commit/a105b4e7a654a7c30eb21de6652f00c32b08ee83))
* **server:** supervise the UI server and only clean up our own orphan ([#28](https://github.com/runkids/skillshare-app/issues/28)) ([1776959](https://github.com/runkids/skillshare-app/commit/17769593974262d13dd4b75fda8f413dc8243cb0)), closes [#16](https://github.com/runkids/skillshare-app/issues/16)
* **sync:** auto-sync when skill source files change ([#24](https://github.com/runkids/skillshare-app/issues/24)) ([f61afc2](https://github.com/runkids/skillshare-app/commit/f61afc23b17be6c84fb8cbc9a920016466d030f6)), closes [#10](https://github.com/runkids/skillshare-app/issues/10)
* **update:** check installed skills for updates in the background ([#25](https://github.com/runkids/skillshare-app/issues/25)) ([0d1e57b](https://github.com/runkids/skillshare-app/commit/0d1e57b43c6fda629302cfd23f6c64ca8cf169fd))


### Bug Fixes

* platform PATH separator, dead code removal, and stricter CI ([#26](https://github.com/runkids/skillshare-app/issues/26)) ([c027f6a](https://github.com/runkids/skillshare-app/commit/c027f6a4397eea7fda0edccf2ff09cdc7171798a))
* **webview:** open CLI UI links in the browser and save its downloads ([#23](https://github.com/runkids/skillshare-app/issues/23)) ([a88b492](https://github.com/runkids/skillshare-app/commit/a88b49246d7e2c4c26e0e5706f060f5979dd766f))

## [0.0.13](https://github.com/runkids/skillshare-app/compare/v0.0.12...v0.0.13) (2026-10-01)


### Features

* report what tray Quick Sync did and keep the tray label current ([#18](https://github.com/runkids/skillshare-app/issues/18)) ([03b22a0](https://github.com/runkids/skillshare-app/commit/03b22a0029af21d12271aa9bc1b39d09e9ec515c))
* **update:** restart the app automatically after installing an update ([#19](https://github.com/runkids/skillshare-app/issues/19)) ([6dccee6](https://github.com/runkids/skillshare-app/commit/6dccee622dc6025ba1bb0596088a98af06953fc9))


### Bug Fixes

* **cli:** restart the UI server when upgrading the CLI from Settings ([#17](https://github.com/runkids/skillshare-app/issues/17)) ([389eb1d](https://github.com/runkids/skillshare-app/commit/389eb1d7e563629315762e70d4a3b6fe5c6458e5))
* **projects:** never lose projects.json to a bad read or partial write ([#16](https://github.com/runkids/skillshare-app/issues/16)) ([0098ff8](https://github.com/runkids/skillshare-app/commit/0098ff883961c5dcfdcfe57b85348e191c1ddb8a))
* **tray:** use a monochrome template icon in the macOS menu bar ([#20](https://github.com/runkids/skillshare-app/issues/20)) ([5915c19](https://github.com/runkids/skillshare-app/commit/5915c1902d75acc4aec7bd99eef66e6111373c88))

## [0.0.12](https://github.com/runkids/skillshare-app/compare/v0.0.11...v0.0.12) (2026-10-01)


### Bug Fixes

* keep logs and give the CLI server the user's PATH ([#13](https://github.com/runkids/skillshare-app/issues/13)) ([d5766ad](https://github.com/runkids/skillshare-app/commit/d5766adaa7be8d5c74bbc6fa9e7032959f813b88))

## [0.0.11](https://github.com/runkids/skillshare-app/compare/v0.0.10...v0.0.11) (2026-10-01)


### Features

* check for app and CLI updates and make the app's CLI usable in terminals ([#10](https://github.com/runkids/skillshare-app/issues/10)) ([574c676](https://github.com/runkids/skillshare-app/commit/574c676ea4de75b7690a039f468268626c2f0fea))


### Bug Fixes

* **server:** only kill orphaned skillshare servers listening on our ports ([#12](https://github.com/runkids/skillshare-app/issues/12)) ([e0a4e3a](https://github.com/runkids/skillshare-app/commit/e0a4e3a9440c696e391675955499e0167071f027))

## [0.0.10](https://github.com/runkids/skillshare-app/compare/v0.0.9...v0.0.10) (2026-09-30)


### Features

* **onboarding:** show how to add the CLI to PATH after setup ([#8](https://github.com/runkids/skillshare-app/issues/8)) ([3ff34e1](https://github.com/runkids/skillshare-app/commit/3ff34e10ed53f68d832122de02cb6daebd95ff64))

## [0.0.9](https://github.com/runkids/skillshare-app/compare/v0.0.8...v0.0.9) (2026-09-30)


### Bug Fixes

* unbreak release build and make typecheck real ([#6](https://github.com/runkids/skillshare-app/issues/6)) ([b32b9ab](https://github.com/runkids/skillshare-app/commit/b32b9ab94e829f9e1ffea91083c3c6c68cad879a))

## [0.0.8](https://github.com/runkids/skillshare-app/compare/v0.0.7...v0.0.8) (2026-09-30)


### Features

* refresh shell UI and first-run onboarding ([#4](https://github.com/runkids/skillshare-app/issues/4)) ([e23b66f](https://github.com/runkids/skillshare-app/commit/e23b66f08274df12e614f6610e17fe85f6a57b54))

## [0.0.7](https://github.com/runkids/skillshare-app/compare/v0.0.6...v0.0.7) (2026-09-30)


### Bug Fixes

* find draft releases before preparing release builds ([b0c2995](https://github.com/runkids/skillshare-app/commit/b0c29958f0850ace4c265e4d58ccb6b6f1f56166))

## [0.0.6](https://github.com/runkids/skillshare-app/compare/v0.0.5...v0.0.6) (2026-09-30)


### Features

* run desktop development in Docker and align app integration ([16394aa](https://github.com/runkids/skillshare-app/commit/16394aabae826e7cabf8863e5a7e6ef7484e770c))
