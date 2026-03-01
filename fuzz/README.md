# k7z Fuzz Targets

This directory contains `cargo-fuzz` targets for parser and path-safety hardening.

## Targets

- `safe_join`: fuzz `k7z_common::safe_join` for path traversal and panic resistance
- `detect_format`: fuzz `k7z_common::detect_format_from_path` for parser robustness
- `zip_list`: fuzz `k7z_format_zip::list_from_reader` for ZIP parser robustness

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
