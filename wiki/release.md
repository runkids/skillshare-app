# Releasing

How a version goes from `main` to users, and how to recover when a step fails. The `skillshare-app-release` skill walks through these steps one by one. `README.md` under "Releasing" has the human summary and the list of required secrets.

## Pipeline

- Conventional Commits on `main` drive **Release Please**. It keeps one open PR,
  `chore(main): release X.Y.Z`, that bumps `package.json`, `src-tauri/tauri.conf.json`,
  `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` and `.github/release-please-manifest.json`,
  and writes `CHANGELOG.md`.
- Below 1.0: `feat` → minor, `fix` → patch, breaking → minor. `ci`, `docs`, `chore`,
  `refactor` and `test` commits alone produce no release PR.
- Merging the release PR makes the **Release Please** workflow create the tag and a
  draft, then call `release.yml` *inside the same run*. There is no separate
  `release.yml` run to look for.
- `release.yml` builds macOS (aarch64), Windows and Linux. When all three pass,
  `release-ready` publishes the release as Latest and the `update-homebrew` job
  updates `runkids/homebrew-tap`. A failed build leaves only a draft.
- A tag, once created, uses up its version number even if the build fails. That is
  why v0.0.6 and v0.0.8 were never published. Don't try to reuse a version.

## Choosing a version

Release Please honours a `Release-As: X.Y.Z` line in the body of a commit on `main`. With squash merges, that is the merge body (`gh pr merge <n> --squash --body "Release-As: 0.2.0"`). Another way is to set `"release-as": "X.Y.Z"` under `packages["."]` in `.github/release-please-config.json`. After that release, remove it, or Release Please keeps proposing the same version. Never edit the version fields by hand, and never run `pnpm bump` for the automated flow.

## Recovery

- **A platform build failed:**
  1. Read the log: `gh run view <id> --log-failed`.
  2. A transient failure (runner, network, notarization timeout) can be retried
     with `gh run rerun <id> --failed`. A rerun uploads to the same draft and
     publishes it once every build passes. Right after `rerun`, `gh run watch`
     can return at once, before the rerun has started. Poll until
     `gh run view <id> --json status -q .status` says `completed` instead.
     Example: v0.0.17's Linux upload failed with `other side closed`, and one
     rerun fixed it. The `could not find Cargo.toml` line in the same log is
     harmless and also appears in good runs.
  3. A real code problem needs a fix PR, then a new release. The failed version
     stays a draft. Tell the user it was skipped and why. Don't move or delete the
     tag.
- **A tag exists but there is no run** (a manual tag push, or a retry from scratch):
  `gh workflow run release.yml -f tag_name=vX.Y.Z`.
- **Published, but the Homebrew job did not run or failed:** `update-homebrew.yml`
  also fires on `release: published` when a *person* publishes, so set the release
  back to draft and publish it again with the user's `gh`:
  `gh release edit vX.Y.Z --draft=true`, then
  `gh release edit vX.Y.Z --draft=false --latest`. A release published by the
  workflow's own token starts no other workflow; that is why `release.yml` calls
  the Homebrew job directly.
- **Never** delete published releases or tags, force-push `main`, or rewrite
  history to "fix" a version. Ask the user first.

