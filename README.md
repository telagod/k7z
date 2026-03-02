# k7z

`k7z` is a Rust-first archive tool focused on Linux workflows with strong `7z + zstd` support.

## Build

```bash
cargo build --workspace
```

## Compatibility Corpus

```bash
./scripts/compat-corpus.sh
```

Current corpus scope:
- system tool -> `k7z`: `7z`, `zip`, `tar`, `tar.zst`, `zst`
- `k7z` -> system tool: `7z`, `zip`, `tar.zst`, `zst`

## Fuzzing

```bash
cargo install cargo-fuzz --locked
rustup toolchain install nightly --profile minimal
cd fuzz
cargo +nightly fuzz list
cargo +nightly fuzz run safe_join -- -max_total_time=60
```

Seed corpus is tracked in `fuzz/corpus/<target>/`.

Crash triage helper:

```bash
./scripts/fuzz-triage.sh replay zip_list fuzz/artifacts/zip_list/<crash-file>
```

## CLI

```bash
# pack
k7z pack ./data -o backup.7z --solid -p secret
k7z pack ./data -o backup.tar.zst
k7z pack ./file.bin -o file.bin.zst

# unpack
k7z unpack backup.7z -o ./restore -p secret

# list / test
k7z list backup.7z -p secret
k7z list backup.zip --json
k7z test backup.tar.zst

# benchmark
k7z bench ./data -f 7z -n 5 --level 9 --solid
k7z bench ./data -f tar.zst --warmup 1 -n 3 --json
k7z bench ./data -f zip -n 5 --out ./bench/zip-report.json
k7z bench ./data -f zip -n 5 --csv ./bench/history.csv
k7z bench ./data -f zip -n 5 --jsonl ./bench/history.jsonl
```

7z-style aliases are included:

- `k7z a ...` => `pack`
- `k7z x ...` => `unpack`
- `k7z l ...` => `list`
- `k7z t ...` => `test`

Benchmark command:

- `k7z bench <source> -f <format> [--warmup N] [-n iterations] [--level N] [--solid] [-p password] [--json] [--out file] [--csv file] [--jsonl file]`

## Current format support

- `7z` (pack/unpack/list/test, password read/write)
- `zip` (pack/unpack/list/test, no encryption in this version)
- `tar`, `tar.gz`, `tar.xz`, `tar.zst` (pack/unpack/list/test)
- `zst` single-file stream (pack/unpack/list/test)
