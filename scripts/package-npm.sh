#!/usr/bin/env sh
set -eu

if [ "$#" -ne 3 ]; then
  echo "Usage: scripts/package-npm.sh <artifacts-dir> <output-dir> <version>" >&2
  exit 1
fi

artifacts_dir=$1
output_dir=$2
version=$3
script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
project_dir=$(CDPATH= cd "$script_dir/.." && pwd)
project_version=$(awk -F ' *= *' '$1 == "version" { gsub(/"/, "", $2); print $2; exit }' "$project_dir/Cargo.toml")

if [ "$version" != "$project_version" ]; then
  echo "Package version $version does not match Cargo.toml version $project_version" >&2
  exit 1
fi

if [ -e "$output_dir" ]; then
  echo "Output path already exists: $output_dir" >&2
  exit 1
fi

mkdir -p "$output_dir/bin" "$output_dir/vendor"
cp "$project_dir/npm/package.json" "$output_dir/package.json"
cp "$project_dir/npm/bin/ggx" "$output_dir/bin/ggx"
cp "$project_dir/docs/README.md" "$output_dir/README.md"
cp "$project_dir/docs/LICENSE" "$output_dir/LICENSE"
chmod +x "$output_dir/bin/ggx"

for target in \
  aarch64-apple-darwin \
  aarch64-unknown-linux-gnu \
  x86_64-apple-darwin \
  x86_64-unknown-linux-gnu
do
  archive="$artifacts_dir/ggx-$target.tar.xz"
  if [ ! -f "$archive" ]; then
    echo "Missing release archive: $archive" >&2
    exit 1
  fi

  target_dir="$output_dir/vendor/$target"
  mkdir -p "$target_dir"
  tar -xJf "$archive" -C "$target_dir" --strip-components=1 "$target/ggx"
  chmod +x "$target_dir/ggx"
done

(
  cd "$output_dir"
  npm version "$version" --allow-same-version --no-git-tag-version --package-lock=false >/dev/null
)
