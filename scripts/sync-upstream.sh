#!/usr/bin/env bash

set -euo pipefail

REMOTE="${1:-upstream}"
BASE_BRANCH="${2:-master}"
FEATURE_BRANCH="${3:-on_entity_property_changed}"

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "Not inside a git repository." >&2
  exit 1
fi

CURRENT_BRANCH="$(git branch --show-current)"

if [[ "${CURRENT_BRANCH}" != "${FEATURE_BRANCH}" ]]; then
  echo "Current branch is '${CURRENT_BRANCH}', expected '${FEATURE_BRANCH}'." >&2
  echo "Switch to the feature branch first, or pass a different branch as the third argument." >&2
  exit 1
fi

if ! git remote get-url "${REMOTE}" >/dev/null 2>&1; then
  echo "Remote '${REMOTE}' is not configured." >&2
  echo "Add it first, for example:" >&2
  echo "  git remote add upstream <UPSTREAM_URL>" >&2
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Working tree is not clean. Commit or stash changes before syncing upstream." >&2
  exit 1
fi

echo "Fetching ${REMOTE}..."
git fetch "${REMOTE}"

echo "Rebasing ${FEATURE_BRANCH} onto ${REMOTE}/${BASE_BRANCH}..."
git rebase "${REMOTE}/${BASE_BRANCH}"

echo "Running validation..."
cargo check
cargo test --lib

echo
echo "Sync complete."
echo "Branch: ${FEATURE_BRANCH}"
echo "Base: ${REMOTE}/${BASE_BRANCH}"
