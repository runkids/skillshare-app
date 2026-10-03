---
name: skillshare-app-pr
description: >-
  Take a change to the skillshare desktop app (runkids/skillshare-app) from request to
  merged PR: worktree branch, implementation, the full CI checks, a screenshot of the
  running app in the dev container, a Conventional Commit, a PR, and a merge only on
  green CI. Use for any code, UI, icon or doc change the user wants landed in
  skillshare-app — "改", "加", "拔掉", "修", "fix this", "add a button", "open a PR",
  "merge it", "上 main" — even when they don't mention PRs. Shipping a version
  afterwards is the separate skillshare-app-release skill.
---

# Land a change in skillshare-app

The facts and rules live in the wiki. This skill only orders the steps. Before step 1,
load `python3 scripts/ai-context.py development`, plus the one topic for the area you
are changing (`frontend`, `architecture`, `background-services`).

## Steps

### 1. Branch

1. Run `git fetch` and check `git status` in the main checkout. Leave the user's
   uncommitted work alone.
2. Create a worktree on a new branch from `origin/main`, as "Shipping a change"
   describes.
3. If several requests arrive together, handle them in one PR only when they touch the
   same files. Otherwise use one PR each.

### 2. Change

1. Before editing, `rg` every use of anything you will change or remove: commands, the
   bridge, settings fields, capabilities, tests, wiki and README.
2. When you remove a feature, remove all of it: UI, Rust commands, plugins,
   capabilities, CliMeta fields, lockfiles, tests and docs. Then `rg` again until
   nothing is left.
3. Update the wiki topics and README that describe the old behavior in the same change.

### 3. Verify

1. Run every command in the "Verifying a change" table and fix whatever fails.
2. For anything visible, apply the branch to the main checkout and take screenshots in
   noVNC. Read each image before you call it verified. Then revert the main checkout.
3. If the user sends a screenshot, read it first. Convert it with `sips` when it is not
   a real PNG.

### 4. Pull request

1. Commit, push and run `gh pr create` with the What/Verification body.
2. The user has delegated merging, so run `scripts/merge-pr.sh <pr>` in the background
   and keep working.
3. On a real failure, read `gh run view <id> --log-failed`, fix it, push a new commit,
   and run the script again.

### 5. Finish

1. Pull `main` into the main checkout and remove the worktree.
2. Tell the user, in their language:
   - what changed and the PR link
   - what was verified, and how: tests or screenshots
   - anything assumed or left out
3. Offer a release in one line. Release only when they ask, using
   `skillshare-app-release`.

## Rules

- Never `--no-verify`. Never force-push or amend a pushed commit; merge `origin/main`
  into the branch instead.
- Report a failing check with its output. Never merge on red.
- One PR, one purpose. The commit body says why.
