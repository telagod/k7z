SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help

FUZZ_TARGETS := safe_join detect_format zip_list tar_list zstd_list
FUZZ_TARGET ?=
FUZZ_INPUT ?=
FUZZ_SECONDS ?= 60
RELEASE_VERSION ?=
RELEASE_REMOTE ?= origin

.PHONY: help fuzz-list fuzz-run fuzz-smoke-all fuzz-dispatch fuzz-triage-replay fuzz-triage-tmin fuzz-triage-both release-check release-rc release-stable release-rc-dryrun release-stable-dryrun release-latest-run release-watch

help:
	@printf '%s\n' \
	  'k7z Make targets:' \
	  '  make fuzz-list' \
	  '  make fuzz-run FUZZ_TARGET=<target> [FUZZ_SECONDS=60]' \
	  '  make fuzz-smoke-all' \
	  '  make fuzz-dispatch [FUZZ_SECONDS=180]' \
	  '  make fuzz-triage-replay FUZZ_TARGET=<target> FUZZ_INPUT=<path>' \
	  '  make fuzz-triage-tmin   FUZZ_TARGET=<target> FUZZ_INPUT=<path>' \
	  '  make fuzz-triage-both   FUZZ_TARGET=<target> FUZZ_INPUT=<path>' \
	  '  make release-check  RELEASE_VERSION=<x.y.z[-rc.N]>' \
	  '  make release-rc     RELEASE_VERSION=<x.y.z-rc.N> [RELEASE_REMOTE=origin]' \
	  '  make release-stable RELEASE_VERSION=<x.y.z> [RELEASE_REMOTE=origin]' \
	  '  make release-rc-dryrun     RELEASE_VERSION=<x.y.z-rc.N>' \
	  '  make release-stable-dryrun RELEASE_VERSION=<x.y.z>' \
	  '  make release-latest-run' \
	  '  make release-watch'

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

release-check:
	@set -euo pipefail; \
	test -n "$(RELEASE_VERSION)" || { echo "missing RELEASE_VERSION" >&2; exit 2; }; \
	if ! [[ "$(RELEASE_VERSION)" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$$ ]]; then \
	  echo "invalid RELEASE_VERSION: $(RELEASE_VERSION)" >&2; \
	  echo "expected: x.y.z or x.y.z-rc.N" >&2; \
	  exit 2; \
	fi; \
	if [[ -n "$$(git status --porcelain)" ]]; then \
	  echo "working tree is not clean" >&2; \
	  exit 1; \
	fi; \
	branch="$$(git branch --show-current)"; \
	if [[ "$$branch" != "main" ]]; then \
	  echo "release tags must be created from main (current: $$branch)" >&2; \
	  exit 1; \
	fi; \
	tag="v$(RELEASE_VERSION)"; \
	if git rev-parse -q --verify "refs/tags/$$tag" >/dev/null; then \
	  echo "local tag already exists: $$tag" >&2; \
	  exit 1; \
	fi; \
	if git ls-remote --tags "$(RELEASE_REMOTE)" "refs/tags/$$tag" | grep -q .; then \
	  echo "remote tag already exists on $(RELEASE_REMOTE): $$tag" >&2; \
	  exit 1; \
	fi; \
	echo "[release] checks passed for $$tag"

release-rc:
	@test -n "$(RELEASE_VERSION)" || (echo "missing RELEASE_VERSION" >&2; exit 2)
	@if [[ "$(RELEASE_VERSION)" != *-* ]]; then \
	  echo "release-rc requires prerelease version (example: 0.1.0-rc.3)" >&2; \
	  exit 2; \
	fi
	@$(MAKE) release-check RELEASE_VERSION="$(RELEASE_VERSION)" RELEASE_REMOTE="$(RELEASE_REMOTE)"
	@tag="v$(RELEASE_VERSION)"; \
	git tag -a "$$tag" -m "Release $$tag"; \
	git push "$(RELEASE_REMOTE)" "$$tag"; \
	echo "[release] pushed $$tag to $(RELEASE_REMOTE)"

release-stable:
	@test -n "$(RELEASE_VERSION)" || (echo "missing RELEASE_VERSION" >&2; exit 2)
	@if [[ "$(RELEASE_VERSION)" == *-* ]]; then \
	  echo "release-stable requires stable version (example: 0.1.0)" >&2; \
	  exit 2; \
	fi
	@$(MAKE) release-check RELEASE_VERSION="$(RELEASE_VERSION)" RELEASE_REMOTE="$(RELEASE_REMOTE)"
	@tag="v$(RELEASE_VERSION)"; \
	git tag -a "$$tag" -m "Release $$tag"; \
	git push "$(RELEASE_REMOTE)" "$$tag"; \
	echo "[release] pushed $$tag to $(RELEASE_REMOTE)"

release-rc-dryrun:
	@test -n "$(RELEASE_VERSION)" || (echo "missing RELEASE_VERSION" >&2; exit 2)
	@if [[ "$(RELEASE_VERSION)" != *-* ]]; then \
	  echo "release-rc-dryrun requires prerelease version (example: 0.1.0-rc.3)" >&2; \
	  exit 2; \
	fi
	@$(MAKE) release-check RELEASE_VERSION="$(RELEASE_VERSION)" RELEASE_REMOTE="$(RELEASE_REMOTE)"
	@echo "[release-dryrun] would create and push tag v$(RELEASE_VERSION) to $(RELEASE_REMOTE)"

release-stable-dryrun:
	@test -n "$(RELEASE_VERSION)" || (echo "missing RELEASE_VERSION" >&2; exit 2)
	@if [[ "$(RELEASE_VERSION)" == *-* ]]; then \
	  echo "release-stable-dryrun requires stable version (example: 0.1.0)" >&2; \
	  exit 2; \
	fi
	@$(MAKE) release-check RELEASE_VERSION="$(RELEASE_VERSION)" RELEASE_REMOTE="$(RELEASE_REMOTE)"
	@echo "[release-dryrun] would create and push tag v$(RELEASE_VERSION) to $(RELEASE_REMOTE)"

release-latest-run:
	@run="$$(gh run list --repo telagod/k7z --workflow Release --limit 1 \
	  --json databaseId,status,conclusion,event,headSha,url,createdAt,updatedAt \
	  --jq 'if length == 0 then "" else "id=\(.[0].databaseId) status=\(.[0].status) conclusion=\((.[0].conclusion // "n/a")) event=\(.[0].event) sha=\(.[0].headSha[0:7]) url=\(.[0].url)" end')"; \
	if [[ -z "$$run" ]]; then \
	  echo "[release] no workflow runs found"; \
	else \
	  echo "$$run"; \
	fi

release-watch:
	@run_id="$$(gh run list --repo telagod/k7z --workflow Release --limit 1 \
	  --json databaseId --jq 'if length == 0 then "" else .[0].databaseId end')"; \
	if [[ -z "$$run_id" ]]; then \
	  echo "[release] no workflow runs found"; \
	  exit 1; \
	fi; \
	echo "[release] watching run $$run_id"; \
	gh run watch --repo telagod/k7z --exit-status "$$run_id"; \
	gh run view --repo telagod/k7z "$$run_id" \
	  --json databaseId,status,conclusion,event,headSha,url \
	  --jq '"id=\(.databaseId) status=\(.status) conclusion=\((.conclusion // "n/a")) event=\(.event) sha=\(.headSha[0:7]) url=\(.url)"'
