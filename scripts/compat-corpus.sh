#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT

input_root="${tmpdir}/input"
mkdir -p "${input_root}/tree/sub"
printf 'alpha\n' > "${input_root}/tree/a.txt"
printf 'beta\n' > "${input_root}/tree/sub/b.txt"
printf 'single-zstd\n' > "${input_root}/single.txt"

run_k7z() {
  cargo run -q -p k7z-cli -- "$@"
}

assert_dir_eq() {
  local left="$1"
  local right="$2"
  diff -ruN "${left}" "${right}"
}

assert_file_eq() {
  local left="$1"
  local right="$2"
  cmp -s "${left}" "${right}"
}

cd "${repo_root}"

echo "[compat] system -> k7z"
mkdir -p "${tmpdir}/system-src"
cp -r "${input_root}/tree" "${tmpdir}/system-src/"
(
  cd "${tmpdir}/system-src"
  7z a -bd "${tmpdir}/system.7z" tree >/dev/null
  zip -rq "${tmpdir}/system.zip" tree
  tar -cf "${tmpdir}/system.tar" tree
  tar --zstd -cf "${tmpdir}/system.tar.zst" tree
)
zstd -q -f "${input_root}/single.txt" -o "${tmpdir}/system-single.txt.zst"

run_k7z unpack "${tmpdir}/system.7z" -o "${tmpdir}/out-system-7z"
run_k7z unpack "${tmpdir}/system.zip" -o "${tmpdir}/out-system-zip"
run_k7z unpack "${tmpdir}/system.tar" -o "${tmpdir}/out-system-tar"
run_k7z unpack "${tmpdir}/system.tar.zst" -o "${tmpdir}/out-system-tarzst"
run_k7z unpack "${tmpdir}/system-single.txt.zst" -o "${tmpdir}/out-system-zst"

assert_dir_eq "${tmpdir}/system-src/tree" "${tmpdir}/out-system-7z/tree"
assert_dir_eq "${tmpdir}/system-src/tree" "${tmpdir}/out-system-zip/tree"
assert_dir_eq "${tmpdir}/system-src/tree" "${tmpdir}/out-system-tar/tree"
assert_dir_eq "${tmpdir}/system-src/tree" "${tmpdir}/out-system-tarzst/tree"
assert_file_eq "${input_root}/single.txt" "${tmpdir}/out-system-zst/system-single.txt"

echo "[compat] k7z -> system"
run_k7z pack "${input_root}/tree" -o "${tmpdir}/k7z.7z"
run_k7z pack "${input_root}/tree" -o "${tmpdir}/k7z.zip"
run_k7z pack "${input_root}/tree" -o "${tmpdir}/k7z.tar.zst"
run_k7z pack "${input_root}/single.txt" -o "${tmpdir}/k7z-single.txt.zst"

mkdir -p "${tmpdir}/out-k7z-7z" "${tmpdir}/out-k7z-zip" "${tmpdir}/out-k7z-tarzst" "${tmpdir}/out-k7z-zst"
7z x -y -bd "${tmpdir}/k7z.7z" "-o${tmpdir}/out-k7z-7z" >/dev/null
unzip -q "${tmpdir}/k7z.zip" -d "${tmpdir}/out-k7z-zip"
tar --zstd -xf "${tmpdir}/k7z.tar.zst" -C "${tmpdir}/out-k7z-tarzst"
zstd -q -d -f "${tmpdir}/k7z-single.txt.zst" -o "${tmpdir}/out-k7z-zst/k7z-single.txt"

# k7z 7z backend stores directory contents relative to source root.
assert_dir_eq "${input_root}/tree" "${tmpdir}/out-k7z-7z"
assert_dir_eq "${input_root}/tree" "${tmpdir}/out-k7z-zip/tree"
assert_dir_eq "${input_root}/tree" "${tmpdir}/out-k7z-tarzst/tree"
assert_file_eq "${input_root}/single.txt" "${tmpdir}/out-k7z-zst/k7z-single.txt"

echo "[compat] done"
