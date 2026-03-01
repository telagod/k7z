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
cd fuzz
RUST_BACKTRACE=1 cargo +nightly fuzz run <target> artifacts/<target>/<crash-file>
```

Minimize a crashing input:

```bash
cd fuzz
cargo +nightly fuzz tmin <target> artifacts/<target>/<crash-file>
```
