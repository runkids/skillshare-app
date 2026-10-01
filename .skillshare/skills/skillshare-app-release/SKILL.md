---
name: skillshare-app-release
description: >-
  Release the skillshare desktop app (runkids/skillshare-app): pick or confirm the
  version, merge the Release Please PR, watch the macOS/Windows/Linux build, and
  verify the GitHub release, updater feed and Homebrew cask. Use whenever the user
  wants to ship the app — "發版", "上版", "發一版", "release", "cut 0.x", "ship it",
  "下一版發 0.2.0", or asks to publish, retry or check a release, even without
  naming the skill. Not for the skillshare CLI repo, which has its own release flow.
---

# Release the skillshare app

The release is mostly automated. Your job is to start it at the right moment,
choose the version when asked, and confirm that every part actually landed.

## How the pipeline works

Knowing this saves you from watching the wrong thing:

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

## Steps

### 1. Preflight

```bash
git fetch --tags origin
gh pr list --state open --json number,title,headRefName
gh run list --branch main --workflow ci.yml --limit 1 --json databaseId,headSha,status,conclusion
```

Releasing happens on GitHub, so the local working tree doesn't matter. Work from
`origin/main`, and don't switch branches, stash or pull over the user's local changes.

- Only the release PR should be open. Other open PRs are not in this release.
  Tell the user, and ask whether to merge them first.
- If main's latest CI run is still in progress, wait for it with `gh run watch <id>`.
  The release PR is opened by a bot, so CI does not run on it. Main's CI is the only
  check on the code being released, so don't merge on a red or unfinished run.

### 2. Find the release PR and settle the version

- **No release PR:**
  1. Check whether Release Please is still running
     (`gh run list --workflow "Release Please" --limit 1`).
  2. If it has finished, list what is unreleased:
     ```bash
     git log --oneline "$(git describe --tags --abbrev=0 origin/main)"..origin/main
     ```
  3. If none of those commits is a `feat` or `fix`, there is nothing to release.
     Say so, and say what would make a release: merge a `feat`/`fix` PR, or use
     `Release-As` (below). Don't invent a release.
- **The user named a version** (e.g. "發 0.2.0"): Release Please honours a
  `Release-As: X.Y.Z` line in a commit body on `main`.
  - If a feature or fix PR is about to be merged anyway, put the line in its
    squash body:
    ```bash
    gh pr merge <n> --squash --admin --delete-branch --body "Release-As: 0.2.0"
    ```
  - Otherwise open a small PR that adds `"release-as": "0.2.0"` to
    `packages["."]` in `.github/release-please-config.json`.
  - Either way, wait for the release PR to show the new version.
  - If you used the config route, remove `release-as` in a follow-up PR after the
    release. Otherwise Release Please keeps asking for that version.
- **Review the release PR diff** (`gh pr diff <n>`):
  - All five version fields agree.
  - The CHANGELOG lists what the user expects.
  - Show the user the version and the CHANGELOG entries in your update.

### 3. Merge

```bash
gh pr merge <n> --squash --admin --delete-branch
```

`main` requires a code-owner review, and the user has delegated merging, so use
`--admin`.

### 4. Watch the build

The build takes about 20–30 minutes. Run it in the background and keep working,
or report, while it runs:

Find the run for the merge commit, not simply the latest run. Right after the merge,
the new run may not exist yet, and `--limit 1` would return the previous release's run:

```bash
sha=$(gh pr view <n> --json mergeCommit -q .mergeCommit.oid)
id=$(gh run list --workflow "Release Please" --commit "$sha" --json databaseId -q '.[0].databaseId')   # empty → wait a few seconds and retry
gh run watch "$id" --exit-status
gh run view "$id" --json jobs -q '.jobs[]|"\(.name): \(.conclusion)"'
```

### 5. Verify that it landed

Check every one of these. A green run alone is not proof that users can update.

```bash
gh release view vX.Y.Z --json isDraft,assets -q '"draft=\(.isDraft) assets=\(.assets|length)"'   # draft=false, 20 assets
gh release list --limit 1                                                                          # vX.Y.Z  Latest
curl -sL https://github.com/runkids/skillshare-app/releases/latest/download/latest.json | jq -r .version
gh run view "$id" --json jobs -q '.jobs[]|select(.name|test("homebrew"))|"\(.name): \(.conclusion)"'
```

The Homebrew line must say `success`. Empty output means the job never ran, which
is a failure, not a pass. Look for a standalone run with
`gh run list --workflow update-homebrew.yml --limit 3`, then see "When something
fails".

If the user asks for a check in the real app, use **About → Check for Updates** in
an installed older build. Updating should install the new version and restart the app.

## When something fails

- **A platform build failed:**
  1. Read the log: `gh run view <id> --log-failed`.
  2. A transient failure (runner, network, notarization timeout) can be retried
     with `gh run rerun <id> --failed`. A rerun uploads to the same draft and
     publishes it once every build passes.
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

## Report

End with a short report in the user's language (繁體中文 for this user):

- The version and the release URL.
- What is in it: one line per CHANGELOG entry, written for users.
- Verification: platforms built, Latest, `latest.json` version, Homebrew.
- Anything skipped or failed, and what is still open (for example a pending PR,
  or a fix that missed this release).
