#!/usr/bin/env bash
# Validate commit messages in a git range with Commitizen.
# Usage: check-commits.sh <base>..<head>
#
# Merge commits are skipped. actions/checkout checks out a synthetic merge
# commit for pull requests, and that subject is not a Conventional Commit.

set -euo pipefail

range="${1:?usage: check-commits.sh <git-range>}"

if ! commits="$(git rev-list --no-merges "$range")"; then
  printf 'cannot read commits in range: %s\n' "$range" >&2
  exit 2
fi

if [[ -z "$commits" ]]; then
  printf 'no commits in range: %s\n' "$range"
  exit 0
fi

status=0
while IFS= read -r sha; do
  if ! git log -1 --format=%B "$sha" | cz check; then
    printf 'bad commit: %s\n' "$sha" >&2
    status=1
  fi
done <<< "$commits"

exit "$status"
