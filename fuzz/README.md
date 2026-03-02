# k7z Fuzz Targets

This directory contains `cargo-fuzz` targets for parser and path-safety hardening.

## Targets

- `safe_join`: fuzz `k7z_common::safe_join` for path traversal and panic resistance
- `detect_format`: fuzz `k7z_common::detect_format_from_path` for parser robustness
- `zip_list`: fuzz `k7z_format_zip::list_from_reader` for ZIP parser robustness
- `tar_list`: fuzz `k7z_format_tar::list_from_reader` for TAR parser robustness
- `zstd_list`: fuzz `k7z_format_zstd::list_from_reader` for Zstd stream parser robustness

## Quick Start

```bash
cargo install cargo-fuzz --locked
rustup toolchain install nightly --profile minimal
cd fuzz
cargo +nightly fuzz list
```

Run one target for 60 seconds:

```bash
cargo +nightly fuzz run safe_join -- -max_total_time=60
```

Trigger remote fuzz with custom duration:

```bash
gh workflow run fuzz.yml -f max_total_time=180
```

Equivalent Make aliases:

```bash
make fuzz-list
make fuzz-run FUZZ_TARGET=tar_list FUZZ_SECONDS=60
make fuzz-dispatch FUZZ_SECONDS=180
make release-check RELEASE_VERSION=0.1.0-rc.3
make release-rc-dryrun RELEASE_VERSION=0.1.0-rc.3
make release-latest-run
make release-watch
make release-start-rc RELEASE_VERSION=0.1.0-rc.4
```

## Seed Corpus

Initial seeds are versioned under `fuzz/corpus/<target>/`:

- `fuzz/corpus/safe_join/`
- `fuzz/corpus/detect_format/`
- `fuzz/corpus/zip_list/`
- `fuzz/corpus/tar_list/`
- `fuzz/corpus/zstd_list/`

## Crash Triage

When CI reports a fuzz crash, download the artifact and replay locally:

```bash
./scripts/fuzz-triage.sh replay <target> artifacts/<target>/<crash-file>
```

Minimize a crashing input:

```bash
./scripts/fuzz-triage.sh tmin <target> artifacts/<target>/<crash-file>
```

Run both steps in one command:

```bash
./scripts/fuzz-triage.sh both <target> artifacts/<target>/<crash-file>
```
