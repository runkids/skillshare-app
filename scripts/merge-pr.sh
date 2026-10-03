#!/bin/bash
# merge-pr.sh <pr>: wait for the PR's checks, rerun them when only the apt mirror
# timed out (up to 3 times), and squash-merge with --admin only when every check is green.
# Exits non-zero, without merging, on a real failure.
set -u
pr=${1:?usage: scripts/merge-pr.sh <pr>}
cd "$(dirname "$0")/.."
repo=$(gh repo view --json nameWithOwner -q .nameWithOwner)

# A fresh push has no check runs yet; `gh pr checks --watch` would pass on nothing.
sleep 15
sha=$(gh pr view "$pr" --json headRefOid -q .headRefOid)
until [ "$(gh api "repos/$repo/commits/$sha/check-runs" -q .total_count 2>/dev/null || echo 0)" -gt 0 ]; do
  sleep 10
done

for i in 1 2 3 4; do
  gh pr checks "$pr" --watch >/dev/null 2>&1 && break
  run=$(gh pr checks "$pr" --json state,link -q '.[]|select(.state=="FAILURE")|.link' | head -1 |
    sed -E 's#.*/runs/([0-9]+)/.*#\1#')
  if ! gh run view "$run" --log-failed 2>&1 | grep -q "Install Linux dependencies.*timed out"; then
    echo "#$pr: real failure in run $run"
    gh pr checks "$pr"
    exit 1
  fi
  [ "$i" = 4 ] && { echo "#$pr: apt timeouts persist"; exit 1; }
  echo "#$pr: apt timeout, rerun $i"
  gh run rerun "$run" --failed
  sleep 30
done

gh pr checks "$pr" | awk '{print $1, $2}'
# GitHub sometimes refuses a merge for a few seconds after the checks finish.
for _ in 1 2 3; do
  gh pr merge "$pr" --squash --admin --delete-branch >/dev/null 2>&1
  [ "$(gh pr view "$pr" --json state -q .state)" = MERGED ] && { echo "#$pr MERGED"; exit 0; }
  sleep 10
done
echo "#$pr: merge failed"
exit 1
