SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help

FUZZ_TARGETS := safe_join detect_format zip_list tar_list zstd_list
FUZZ_TARGET ?=
FUZZ_INPUT ?=
FUZZ_SECONDS ?= 60
RELEASE_VERSION ?=
RELEASE_REMOTE ?= origin
RELEASE_TAG ?=
EXPECT_PRERELEASE ?= auto

.PHONY: help fuzz-list fuzz-run fuzz-smoke-all fuzz-dispatch fuzz-triage-replay fuzz-triage-tmin fuzz-triage-both release-check release-rc release-stable release-rc-dryrun release-stable-dryrun release-latest-run release-show release-verify-assets release-promote-check release-watch release-start-rc release-start-stable release-start-rc-dryrun release-start-stable-dryrun release-start-rc-verified release-start-stable-verified

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
	  '  make release-show [RELEASE_TAG=<vX.Y.Z> | RELEASE_VERSION=<x.y.z[-rc.N]>]' \
	  '  make release-verify-assets [RELEASE_TAG=<vX.Y.Z> | RELEASE_VERSION=<x.y.z[-rc.N]>]' \
	  '  make release-promote-check [RELEASE_TAG=<vX.Y.Z> | RELEASE_VERSION=<x.y.z[-rc.N]>] [EXPECT_PRERELEASE=auto|true|false]' \
	  '  make release-watch' \
	  '  make release-start-rc     RELEASE_VERSION=<x.y.z-rc.N>' \
	  '  make release-start-stable RELEASE_VERSION=<x.y.z>' \
	  '  make release-start-rc-dryrun     RELEASE_VERSION=<x.y.z-rc.N>' \
	  '  make release-start-stable-dryrun RELEASE_VERSION=<x.y.z>' \
	  '  make release-start-rc-verified     RELEASE_VERSION=<x.y.z-rc.N>' \
	  '  make release-start-stable-verified RELEASE_VERSION=<x.y.z>'

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

release-show:
	@set -euo pipefail; \
	tag="$(RELEASE_TAG)"; \
	if [[ -z "$$tag" && -n "$(RELEASE_VERSION)" ]]; then \
	  tag="v$(RELEASE_VERSION)"; \
	fi; \
	if [[ -z "$$tag" ]]; then \
	  echo "missing RELEASE_TAG (or set RELEASE_VERSION)" >&2; \
	  exit 2; \
	fi; \
	if [[ "$$tag" != v* ]]; then \
	  tag="v$$tag"; \
	fi; \
	gh release view --repo telagod/k7z "$$tag" \
	  --json tagName,isPrerelease,publishedAt,url,assets \
	  --jq '"tag=\(.tagName) prerelease=\(.isPrerelease) published=\(.publishedAt) assets=\(.assets|length) url=\(.url)"'; \
	gh release view --repo telagod/k7z "$$tag" \
	  --json assets \
	  --jq '.assets[]? | "asset=\(.name) size=\(.size) bytes"'

release-verify-assets:
	@set -euo pipefail; \
	tag="$(RELEASE_TAG)"; \
	if [[ -z "$$tag" && -n "$(RELEASE_VERSION)" ]]; then \
	  tag="v$(RELEASE_VERSION)"; \
	fi; \
	if [[ -z "$$tag" ]]; then \
	  echo "missing RELEASE_TAG (or set RELEASE_VERSION)" >&2; \
	  exit 2; \
	fi; \
	if [[ "$$tag" != v* ]]; then \
	  tag="v$$tag"; \
	fi; \
	command -v sha256sum >/dev/null || { echo "sha256sum not found" >&2; exit 1; }; \
	tmpdir="$$(mktemp -d)"; \
	trap 'rm -rf "$$tmpdir"' EXIT; \
	gh release download --repo telagod/k7z "$$tag" \
	  --pattern '*.tar.gz' \
	  --pattern '*.sha256' \
	  --dir "$$tmpdir"; \
	if ! compgen -G "$$tmpdir/*.sha256" >/dev/null; then \
	  echo "[release] no .sha256 assets found for $$tag" >&2; \
	  exit 1; \
	fi; \
	if ! compgen -G "$$tmpdir/*.tar.gz" >/dev/null; then \
	  echo "[release] no .tar.gz assets found for $$tag" >&2; \
	  exit 1; \
	fi; \
	count=0; \
	for checksum in "$$tmpdir"/*.sha256; do \
	  (cd "$$tmpdir" && sha256sum -c "$$(basename "$$checksum")"); \
	  count=$$((count + 1)); \
	done; \
	echo "[release] verified $$count checksum file(s) for $$tag"

release-promote-check:
	@set -euo pipefail; \
	tag="$(RELEASE_TAG)"; \
	if [[ -z "$$tag" && -n "$(RELEASE_VERSION)" ]]; then \
	  tag="v$(RELEASE_VERSION)"; \
	fi; \
	if [[ -z "$$tag" ]]; then \
	  echo "missing RELEASE_TAG (or set RELEASE_VERSION)" >&2; \
	  exit 2; \
	fi; \
	if [[ "$$tag" != v* ]]; then \
	  tag="v$$tag"; \
	fi; \
	expect="$(EXPECT_PRERELEASE)"; \
	if [[ -z "$$expect" || "$$expect" == "auto" ]]; then \
	  if [[ "$$tag" == *-* ]]; then \
	    expect="true"; \
	  else \
	    expect="false"; \
	  fi; \
	fi; \
	if [[ "$$expect" != "true" && "$$expect" != "false" ]]; then \
	  echo "EXPECT_PRERELEASE must be auto|true|false" >&2; \
	  exit 2; \
	fi; \
	actual="$$(gh release view --repo telagod/k7z "$$tag" --json isPrerelease --jq '.isPrerelease')"; \
	if [[ "$$actual" != "$$expect" ]]; then \
	  echo "[release] prerelease mismatch for $$tag: expect=$$expect actual=$$actual" >&2; \
	  exit 1; \
	fi; \
	required_assets=( \
	  "k7z-x86_64-unknown-linux-gnu.tar.gz" \
	  "k7z-x86_64-unknown-linux-gnu.sha256" \
	  "k7z-aarch64-unknown-linux-gnu.tar.gz" \
	  "k7z-aarch64-unknown-linux-gnu.sha256" \
	); \
	for asset in "$${required_assets[@]}"; do \
	  count="$$(gh release view --repo telagod/k7z "$$tag" --json assets --jq '[.assets[] | select(.name=="'"$$asset"'")] | length')"; \
	  if [[ "$$count" -ne 1 ]]; then \
	    echo "[release] missing or duplicated asset for $$tag: $$asset (count=$$count)" >&2; \
	    exit 1; \
	  fi; \
	done; \
	total="$$(gh release view --repo telagod/k7z "$$tag" --json assets --jq '.assets | length')"; \
	echo "[release] promote-check passed for $$tag prerelease=$$actual required_assets=4 total_assets=$$total"

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

release-start-rc:
	@test -n "$(RELEASE_VERSION)" || (echo "missing RELEASE_VERSION" >&2; exit 2)
	@$(MAKE) release-rc RELEASE_VERSION="$(RELEASE_VERSION)" RELEASE_REMOTE="$(RELEASE_REMOTE)"
	@$(MAKE) release-watch

release-start-stable:
	@test -n "$(RELEASE_VERSION)" || (echo "missing RELEASE_VERSION" >&2; exit 2)
	@$(MAKE) release-stable RELEASE_VERSION="$(RELEASE_VERSION)" RELEASE_REMOTE="$(RELEASE_REMOTE)"
	@$(MAKE) release-watch

release-start-rc-dryrun:
	@test -n "$(RELEASE_VERSION)" || (echo "missing RELEASE_VERSION" >&2; exit 2)
	@$(MAKE) release-rc-dryrun RELEASE_VERSION="$(RELEASE_VERSION)" RELEASE_REMOTE="$(RELEASE_REMOTE)"
	@$(MAKE) release-latest-run

release-start-stable-dryrun:
	@test -n "$(RELEASE_VERSION)" || (echo "missing RELEASE_VERSION" >&2; exit 2)
	@$(MAKE) release-stable-dryrun RELEASE_VERSION="$(RELEASE_VERSION)" RELEASE_REMOTE="$(RELEASE_REMOTE)"
	@$(MAKE) release-latest-run

release-start-rc-verified:
	@test -n "$(RELEASE_VERSION)" || (echo "missing RELEASE_VERSION" >&2; exit 2)
	@$(MAKE) release-rc RELEASE_VERSION="$(RELEASE_VERSION)" RELEASE_REMOTE="$(RELEASE_REMOTE)"
	@$(MAKE) release-watch
	@$(MAKE) release-verify-assets RELEASE_VERSION="$(RELEASE_VERSION)"
	@$(MAKE) release-promote-check RELEASE_VERSION="$(RELEASE_VERSION)" EXPECT_PRERELEASE=true

release-start-stable-verified:
	@test -n "$(RELEASE_VERSION)" || (echo "missing RELEASE_VERSION" >&2; exit 2)
	@$(MAKE) release-stable RELEASE_VERSION="$(RELEASE_VERSION)" RELEASE_REMOTE="$(RELEASE_REMOTE)"
	@$(MAKE) release-watch
	@$(MAKE) release-verify-assets RELEASE_VERSION="$(RELEASE_VERSION)"
	@$(MAKE) release-promote-check RELEASE_VERSION="$(RELEASE_VERSION)" EXPECT_PRERELEASE=false
