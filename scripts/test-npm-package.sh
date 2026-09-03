#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
project_dir=$(CDPATH= cd "$script_dir/.." && pwd)
project_version=$(awk -F ' *= *' '$1 == "version" { gsub(/"/, "", $2); print $2; exit }' "$project_dir/Cargo.toml")
npx_command=$(command -v npx)
bunx_command=$(command -v bunx)
test_dir=$(mktemp -d "${TMPDIR:-/tmp}/ggx-npm-test.XXXXXX")
test_dir=$(CDPATH= cd "$test_dir" && pwd)
artifacts_dir="$test_dir/artifacts"
archive_dir="$test_dir/archive"
package_dir="$test_dir/package"
mock_bin="$test_dir/mock-bin"
mkdir -p "$artifacts_dir" "$archive_dir" "$mock_bin"

for target in \
  aarch64-apple-darwin \
  aarch64-unknown-linux-gnu \
  x86_64-apple-darwin \
  x86_64-unknown-linux-gnu
do
  target_dir="$archive_dir/$target"
  mkdir -p "$target_dir"
  cp "$project_dir/scripts/testdata/fake-ggx.sh" "$target_dir/ggx"
  chmod +x "$target_dir/ggx"
  tar -cJf "$artifacts_dir/ggx-$target.tar.xz" -C "$archive_dir" "$target"
done

"$project_dir/scripts/package-npm.sh" "$artifacts_dir" "$package_dir" "$project_version"

node - "$package_dir/package.json" "$project_version" <<'NODE'
const fs = require("fs");
const manifest = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
if (manifest.name !== "ggx-ai") throw new Error("unexpected package name");
if (manifest.version !== process.argv[3]) throw new Error("unexpected package version");
if (manifest.bin.ggx !== "bin/ggx") throw new Error("unexpected bin entry");
if (manifest.scripts) throw new Error("lifecycle scripts are not allowed");
if (manifest.dependencies) throw new Error("runtime dependencies are not allowed");
NODE

pack_report="$test_dir/pack-report.json"
npm pack --dry-run --json "$package_dir" > "$pack_report"
node - "$pack_report" <<'NODE'
const fs = require("fs");
const parsed = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const report = Array.isArray(parsed) ? parsed[0] : parsed["ggx-ai"];
const files = new Set(report.files.map(({ path }) => path));
const expected = [
  "bin/ggx",
  "vendor/aarch64-apple-darwin/ggx",
  "vendor/aarch64-unknown-linux-gnu/ggx",
  "vendor/x86_64-apple-darwin/ggx",
  "vendor/x86_64-unknown-linux-gnu/ggx",
];
for (const path of expected) {
  if (!files.has(path)) throw new Error(`missing package file: ${path}`);
}
NODE

cp "$project_dir/scripts/testdata/fake-uname.sh" "$mock_bin/uname"
chmod +x "$mock_bin/uname"

check_target() {
  os_name=$1
  architecture=$2
  expected_target=$3
  output=$(printf 'stdin-data\n' | CI=1 GGX_TEST_OS="$os_name" GGX_TEST_ARCH="$architecture" PATH="$mock_bin:$PATH" "$package_dir/bin/ggx" first "two words")
  expected=$(printf 'target=%s\narg=<first>\narg=<two words>\nstdin-data' "$expected_target")
  if [ "$output" != "$expected" ]; then
    echo "Unexpected launcher output for $os_name $architecture" >&2
    exit 1
  fi
}

check_target Darwin arm64 aarch64-apple-darwin
check_target Darwin x86_64 x86_64-apple-darwin
check_target Linux aarch64 aarch64-unknown-linux-gnu
check_target Linux x86_64 x86_64-unknown-linux-gnu

set +e
CI=1 GGX_TEST_OS=FreeBSD GGX_TEST_ARCH=x86_64 PATH="$mock_bin:$PATH" "$package_dir/bin/ggx" >"$test_dir/unsupported-out" 2>"$test_dir/unsupported-error"
unsupported_status=$?
set -e
if [ "$unsupported_status" -ne 1 ] || ! grep -q "does not support" "$test_dir/unsupported-error"; then
  echo "Unsupported platforms must fail with a useful error" >&2
  exit 1
fi

set +e
CI=1 GGX_TEST_OS=Linux GGX_TEST_ARCH=x86_64 GGX_TEST_EXIT=7 PATH="$mock_bin:$PATH" "$package_dir/bin/ggx" >/dev/null
exit_status=$?
set -e
if [ "$exit_status" -ne 7 ]; then
  echo "Launcher did not preserve the binary exit status" >&2
  exit 1
fi

tarball_name=$(cd "$test_dir" && npm pack "$package_dir" --silent)
tarball="$test_dir/$tarball_name"
npm_prefix="$test_dir/npm-prefix"
npm install --global --prefix "$npm_prefix" "$tarball" >/dev/null
CI=1 "$npm_prefix/bin/ggx" >/dev/null

bun_home="$test_dir/bun-home"
BUN_INSTALL="$bun_home" bun add --global "$tarball" >/dev/null
CI=1 "$bun_home/bin/ggx" >/dev/null

cp "$project_dir/scripts/testdata/fake-npm.sh" "$mock_bin/npm"
cp "$project_dir/scripts/testdata/fake-bun.sh" "$mock_bin/bun"
chmod +x "$mock_bin/npm" "$mock_bin/bun"

wait_for_update() {
  update_log=$1
  attempts=0
  while [ "$attempts" -lt 50 ]; do
    if [ -s "$update_log" ]; then
      return
    fi
    attempts=$((attempts + 1))
    sleep 0.1
  done
  echo "Timed out waiting for the background update" >&2
  exit 1
}

npm_log="$test_dir/npm-update.log"
GGX_TEST_OS=Linux \
GGX_TEST_ARCH=x86_64 \
GGX_TEST_NPM_PREFIX="$npm_prefix" \
GGX_TEST_BUN_BIN="$bun_home/other-bin" \
GGX_TEST_UPDATE_LOG="$npm_log" \
PATH="$mock_bin:$PATH" \
"$npm_prefix/bin/ggx" >/dev/null
wait_for_update "$npm_log"
grep -qx "npm update --global ggx-ai" "$npm_log"

bun_log="$test_dir/bun-update.log"
GGX_TEST_OS=Linux \
GGX_TEST_ARCH=x86_64 \
GGX_TEST_NPM_PREFIX="$npm_prefix/other-prefix" \
GGX_TEST_BUN_BIN="$bun_home/bin" \
GGX_TEST_UPDATE_LOG="$bun_log" \
GGX_TEST_UPDATE_FAIL=1 \
PATH="$mock_bin:$PATH" \
"$bun_home/bin/ggx" >/dev/null
wait_for_update "$bun_log"
grep -qx "bun update --global --latest ggx-ai" "$bun_log"

local_log="$test_dir/local-update.log"
GGX_TEST_OS=Linux \
GGX_TEST_ARCH=x86_64 \
GGX_TEST_NPM_PREFIX="$npm_prefix" \
GGX_TEST_BUN_BIN="$bun_home/bin" \
GGX_TEST_UPDATE_LOG="$local_log" \
PATH="$mock_bin:$PATH" \
"$package_dir/bin/ggx" >/dev/null
sleep 0.2
if [ -e "$local_log" ]; then
  echo "Local execution must not start a global update" >&2
  exit 1
fi

npx_log="$test_dir/npx-update.log"
GGX_TEST_OS=Linux \
GGX_TEST_ARCH=x86_64 \
GGX_TEST_NPM_PREFIX="$npm_prefix" \
GGX_TEST_BUN_BIN="$bun_home/bin" \
GGX_TEST_UPDATE_LOG="$npx_log" \
PATH="$mock_bin:$PATH" \
"$npx_command" --yes --package="$tarball" -- ggx >/dev/null
sleep 0.2
if [ -e "$npx_log" ]; then
  echo "npx execution must not start a global update" >&2
  exit 1
fi

bunx_log="$test_dir/bunx-update.log"
BUN_INSTALL="$test_dir/bunx-home" \
GGX_TEST_OS=Linux \
GGX_TEST_ARCH=x86_64 \
GGX_TEST_NPM_PREFIX="$npm_prefix" \
GGX_TEST_BUN_BIN="$bun_home/bin" \
GGX_TEST_UPDATE_LOG="$bunx_log" \
PATH="$mock_bin:$PATH" \
"$bunx_command" --package "$tarball" ggx >/dev/null
sleep 0.2
if [ -e "$bunx_log" ]; then
  echo "bunx execution must not start a global update" >&2
  exit 1
fi

echo "npm and Bun package tests passed"
