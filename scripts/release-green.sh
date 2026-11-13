#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Release guard: the commit to release must already be green as a pull request. Fails, with the
# reason, unless SHA has a check run "required" from a pull_request run of .github/workflows/ci.yml
# and the newest such check run completed with success. Main moves only by a fast-forward push of a
# green pull request head (CONTRIBUTING.md), so the tagged commit is that head and carries the check.
# A "required" job of another workflow or event (an old push run) does not count.
#
# Usage: release-green.sh SHA     env: GITHUB_REPOSITORY, GH_TOKEN (checks: read, actions: read)
# release.yml runs it on a tag push and, as a dry run that publishes nothing, on workflow_dispatch.
set -euo pipefail
sha=${1:?usage: release-green.sh SHA}
repo=${GITHUB_REPOSITORY:?GITHUB_REPOSITORY not set}
fail() {
	echo "release refused: $*" >&2
	exit 1
}

# Newest first.
checks=$(gh api --paginate "repos/$repo/commits/$sha/check-runs?check_name=required&filter=all&per_page=100" \
	--jq '.check_runs[]' | jq -s 'sort_by(.started_at) | reverse')
found=
for row in $(jq -r '.[] | select(.html_url | test("/actions/runs/[0-9]+")) | "\(.html_url | capture("/actions/runs/(?<id>[0-9]+)").id):\(.status):\(.conclusion // "none")"' <<<"$checks"); do
	id=${row%%:*}
	rest=${row#*:}
	status=${rest%%:*}
	conclusion=${rest#*:}
	[ "$(gh api "repos/$repo/actions/runs/$id" --jq '"\(.path) \(.event)"')" = ".github/workflows/ci.yml pull_request" ] || continue
	found=$id
	[ "$status" = completed ] || fail "$sha: the check \"required\" of ci.yml (run $id) is $status, not finished"
	[ "$conclusion" = success ] || fail "$sha: the check \"required\" of ci.yml (run $id) concluded $conclusion"
	echo "required: success (ci.yml pull_request run $id)"
	break
done
[ -n "$found" ] || fail "$sha has no check \"required\" from a ci.yml pull_request run: it was never green as a pull request"
echo "$sha is green: required (ci.yml)"
