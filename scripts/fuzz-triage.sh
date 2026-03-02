#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fuzz_dir="${repo_root}/fuzz"
toolchain="${FUZZ_TOOLCHAIN:-nightly}"

usage() {
  cat <<'EOF'
Usage:
  ./scripts/fuzz-triage.sh <mode> <target> <crash-input>

Modes:
  replay  Re-run a crashing input with backtrace enabled
  tmin    Minimize a crashing input with cargo-fuzz tmin
  both    Run replay first, then tmin

Examples:
  ./scripts/fuzz-triage.sh replay zip_list fuzz/artifacts/zip_list/crash-123
  ./scripts/fuzz-triage.sh tmin tar_list crash-123
  FUZZ_TOOLCHAIN=nightly ./scripts/fuzz-triage.sh both zstd_list fuzz/artifacts/zstd_list/crash-xyz
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -ne 3 ]]; then
  usage
  exit 2
fi

mode="$1"
target="$2"
input="$3"

if [[ ! -d "${fuzz_dir}" ]]; then
  echo "[triage] missing fuzz directory: ${fuzz_dir}" >&2
  exit 1
fi

if [[ ! -f "${input}" ]]; then
  candidate="${fuzz_dir}/artifacts/${target}/${input}"
  if [[ -f "${candidate}" ]]; then
    input="${candidate}"
  fi
fi

if [[ ! -f "${input}" ]]; then
  echo "[triage] crash input not found: ${input}" >&2
  exit 1
fi

cd "${fuzz_dir}"

cargo_cmd=(cargo "+${toolchain}" fuzz)
if ! "${cargo_cmd[@]}" list | grep -Fxq "${target}"; then
  echo "[triage] unknown fuzz target: ${target}" >&2
  echo "[triage] available targets:" >&2
  "${cargo_cmd[@]}" list >&2
  exit 1
fi

run_replay() {
  echo "[triage] replay target=${target} input=${input}"
  RUST_BACKTRACE=1 "${cargo_cmd[@]}" run "${target}" "${input}"
}

run_tmin() {
  echo "[triage] tmin target=${target} input=${input}"
  "${cargo_cmd[@]}" tmin "${target}" "${input}"
}

case "${mode}" in
  replay)
    run_replay
    ;;
  tmin)
    run_tmin
    ;;
  both)
    run_replay
    run_tmin
    ;;
  *)
    echo "[triage] invalid mode: ${mode}" >&2
    usage
    exit 2
    ;;
esac
