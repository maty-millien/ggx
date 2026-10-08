#!/usr/bin/env sh
set -eu

echo "Updating Rust"
rustup update

echo "Updating cargo tools"
command -v cargo-upgrade >/dev/null || cargo install cargo-edit
cargo install-update -a

echo "Updating dependencies"
cargo upgrade --incompatible
cargo update

echo "Updating dist version"
dist_version=$(dist --version | cut -d' ' -f2)
perl -pi -e "s/cargo-dist-version = \"[^\"]*\"/cargo-dist-version = \"$dist_version\"/" dist-workspace.toml
perl -pi -e "s|cargo-dist/releases/download/v[^/]*/|cargo-dist/releases/download/v$dist_version/|" .github/workflows/release.yml

echo "Updating GitHub Actions"
for action in $(grep -hoE '[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+@v[0-9]+' .github/workflows/*.yml | cut -d@ -f1 | sort -u); do
  major=$(gh api "repos/$action/releases/latest" --jq .tag_name | cut -d. -f1)
  perl -pi -e "s|\Q$action\E\@v\d+|$action\@$major|g" .github/workflows/*.yml
done

scripts/ci.sh

echo "Running cargo audit"
cargo audit
