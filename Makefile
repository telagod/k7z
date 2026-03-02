SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help

FUZZ_TARGETS := safe_join detect_format zip_list tar_list zstd_list
FUZZ_TARGET ?=
FUZZ_INPUT ?=
FUZZ_SECONDS ?= 60

.PHONY: help fuzz-list fuzz-run fuzz-smoke-all fuzz-dispatch fuzz-triage-replay fuzz-triage-tmin fuzz-triage-both

help:
	@printf '%s\n' \
	  'k7z Make targets:' \
	  '  make fuzz-list' \
	  '  make fuzz-run FUZZ_TARGET=<target> [FUZZ_SECONDS=60]' \
	  '  make fuzz-smoke-all' \
	  '  make fuzz-dispatch [FUZZ_SECONDS=180]' \
	  '  make fuzz-triage-replay FUZZ_TARGET=<target> FUZZ_INPUT=<path>' \
	  '  make fuzz-triage-tmin   FUZZ_TARGET=<target> FUZZ_INPUT=<path>' \
	  '  make fuzz-triage-both   FUZZ_TARGET=<target> FUZZ_INPUT=<path>'

fuzz-list:
	cd fuzz && cargo +nightly fuzz list

fuzz-run:
	@test -n "$(FUZZ_TARGET)" || (echo "missing FUZZ_TARGET" >&2; exit 2)
	cd fuzz && cargo +nightly fuzz run "$(FUZZ_TARGET)" -- -max_total_time=$(FUZZ_SECONDS)

fuzz-smoke-all:
	cd fuzz && for target in $(FUZZ_TARGETS); do \
	  echo "[fuzz-smoke] $$target"; \
	  cargo +nightly fuzz run "$$target" -- -runs=1; \
	done

fuzz-dispatch:
	gh workflow run fuzz.yml --repo telagod/k7z -f max_total_time=$(FUZZ_SECONDS)

fuzz-triage-replay:
	@test -n "$(FUZZ_TARGET)" || (echo "missing FUZZ_TARGET" >&2; exit 2)
	@test -n "$(FUZZ_INPUT)" || (echo "missing FUZZ_INPUT" >&2; exit 2)
	./scripts/fuzz-triage.sh replay "$(FUZZ_TARGET)" "$(FUZZ_INPUT)"

fuzz-triage-tmin:
	@test -n "$(FUZZ_TARGET)" || (echo "missing FUZZ_TARGET" >&2; exit 2)
	@test -n "$(FUZZ_INPUT)" || (echo "missing FUZZ_INPUT" >&2; exit 2)
	./scripts/fuzz-triage.sh tmin "$(FUZZ_TARGET)" "$(FUZZ_INPUT)"

fuzz-triage-both:
	@test -n "$(FUZZ_TARGET)" || (echo "missing FUZZ_TARGET" >&2; exit 2)
	@test -n "$(FUZZ_INPUT)" || (echo "missing FUZZ_INPUT" >&2; exit 2)
	./scripts/fuzz-triage.sh both "$(FUZZ_TARGET)" "$(FUZZ_INPUT)"
