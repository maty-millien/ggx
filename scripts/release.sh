#!/usr/bin/env sh
set -eu

branch="$(git branch --show-current)"
version="$(awk -F ' *= *' '$1 == "version" { gsub(/"/, "", $2); print $2; exit }' Cargo.toml)"
tag="v${version}"

if [ "$branch" != "main" ]; then
  echo "Release must be run from main"
  exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
  echo "Working tree must be clean before release"
  exit 1
fi

git fetch --tags origin

if git rev-parse "$tag" >/dev/null 2>&1; then
  echo "Tag already exists: $tag"
  exit 1
fi

echo "Checking dist release plan"
dist plan --tag "$tag"

echo "Pushing main"
git push origin main

echo "Creating release tag $tag"
git tag "$tag"
git push origin "$tag"

echo "Release started for $tag"
