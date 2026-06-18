#!/usr/bin/env bash

set -euo pipefail

REMOTE="origin"
BRANCH="on_entity_property_changed"
BUMP="patch"
TAG=""
YES="false"

usage() {
  cat <<'EOF'
Usage:
  scripts/release-tag.sh [options] [vX.Y.Z]

Creates and pushes a release tag from the on_entity_property_changed branch.

Options:
  -r, --remote <name>   Remote to read tags from and push to. Default: origin
  -b, --branch <name>   Required current branch. Default: on_entity_property_changed
  --bump <part>         Version part to increment when tag is omitted: patch, minor, major. Default: patch
  -y, --yes             Do not prompt before creating and pushing the tag
  -h, --help            Show this help

Examples:
  scripts/release-tag.sh
  scripts/release-tag.sh --bump minor
  scripts/release-tag.sh v1.1.0
  scripts/release-tag.sh --remote origin --yes v1.1.0
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -r|--remote)
      REMOTE="${2:?Missing value for $1}"
      shift 2
      ;;
    -b|--branch)
      BRANCH="${2:?Missing value for $1}"
      shift 2
      ;;
    --bump)
      BUMP="${2:?Missing value for $1}"
      shift 2
      ;;
    -y|--yes)
      YES="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    v[0-9]*.[0-9]*.[0-9]*)
      if [[ -n "${TAG}" ]]; then
        echo "Only one tag can be provided." >&2
        exit 1
      fi
      TAG="$1"
      shift
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "Not inside a git repository." >&2
  exit 1
fi

CURRENT_BRANCH="$(git branch --show-current)"
if [[ "${CURRENT_BRANCH}" != "${BRANCH}" ]]; then
  echo "Current branch is '${CURRENT_BRANCH}', expected '${BRANCH}'." >&2
  exit 1
fi

if ! git remote get-url "${REMOTE}" >/dev/null 2>&1; then
  echo "Remote '${REMOTE}' is not configured." >&2
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Working tree is not clean. Commit or stash changes before creating a release tag." >&2
  exit 1
fi

case "${BUMP}" in
  patch|minor|major) ;;
  *)
    echo "--bump must be one of: patch, minor, major" >&2
    exit 1
    ;;
esac

echo "Fetching tags from ${REMOTE}..."
git fetch "${REMOTE}" --tags

if [[ -z "${TAG}" ]]; then
  LATEST_TAG="$(
    git ls-remote --tags --refs "${REMOTE}" 'v[0-9]*.[0-9]*.[0-9]*' |
      awk '{ sub("refs/tags/", "", $2); print $2 }' |
      sort -V |
      tail -n 1
  )"

  if [[ -z "${LATEST_TAG}" ]]; then
    echo "No remote semver tags found. Provide a tag manually, for example: scripts/release-tag.sh v1.0.0" >&2
    exit 1
  fi

  VERSION="${LATEST_TAG#v}"
  IFS='.' read -r MAJOR MINOR PATCH <<<"${VERSION}"

  case "${BUMP}" in
    patch)
      PATCH=$((PATCH + 1))
      ;;
    minor)
      MINOR=$((MINOR + 1))
      PATCH=0
      ;;
    major)
      MAJOR=$((MAJOR + 1))
      MINOR=0
      PATCH=0
      ;;
  esac

  TAG="v${MAJOR}.${MINOR}.${PATCH}"
fi

if [[ ! "${TAG}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Tag must match vX.Y.Z, got: ${TAG}" >&2
  exit 1
fi

if git rev-parse -q --verify "refs/tags/${TAG}" >/dev/null; then
  echo "Local tag '${TAG}' already exists." >&2
  exit 1
fi

if git ls-remote --exit-code --tags --refs "${REMOTE}" "${TAG}" >/dev/null 2>&1; then
  echo "Remote tag '${TAG}' already exists on ${REMOTE}." >&2
  exit 1
fi

HEAD_SHA="$(git rev-parse HEAD)"

echo
echo "Release tag: ${TAG}"
echo "Branch:      ${CURRENT_BRANCH}"
echo "Commit:      ${HEAD_SHA}"
echo "Remote:      ${REMOTE}"
echo
echo "This will run:"
echo "  git push ${REMOTE} ${CURRENT_BRANCH}"
echo "  git tag ${TAG} ${HEAD_SHA}"
echo "  git push ${REMOTE} ${TAG}"
echo

if [[ "${YES}" != "true" ]]; then
  read -r -p "Create and push this tag? [y/N] " CONFIRM
  case "${CONFIRM}" in
    y|Y|yes|YES) ;;
    *)
      echo "Aborted."
      exit 0
      ;;
  esac
fi

git push "${REMOTE}" "${CURRENT_BRANCH}"
git tag "${TAG}" "${HEAD_SHA}"
git push "${REMOTE}" "${TAG}"

echo
echo "Pushed ${TAG} to ${REMOTE}."
