#!/usr/bin/env bash
set -euo pipefail

MODE="write"
README_PATH="README.md"
TAG_OVERRIDE=""

usage() {
  cat <<USAGE
Usage: $0 [--check|--write] [--readme <path>] [--tag <vX.Y.Z>]

Options:
  --check          Verify README is up to date without modifying files.
  --write          Update README in place (default).
  --readme <path>  README file to update/check (default: README.md).
  --tag <tag>      Use explicit tag instead of latest git tag.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check)
      MODE="check"
      shift
      ;;
    --write)
      MODE="write"
      shift
      ;;
    --readme)
      README_PATH="${2:-}"
      shift 2
      ;;
    --tag)
      TAG_OVERRIDE="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ! -f "$README_PATH" ]]; then
  echo "README not found: $README_PATH" >&2
  exit 2
fi

LATEST_TAG="$TAG_OVERRIDE"
if [[ -z "$LATEST_TAG" ]]; then
  LATEST_TAG=$(git tag --list 'v*' --sort=-v:refname | head -n 1 || true)
fi

if [[ -z "$LATEST_TAG" ]]; then
  echo "No v* tag found; skipping README install-version sync."
  exit 0
fi

if [[ "$LATEST_TAG" != v* ]]; then
  echo "Tag must start with 'v': $LATEST_TAG" >&2
  exit 2
fi

EXPECTED_ASSET="dbtl-${LATEST_TAG}-x86_64-unknown-linux-gnu.tar.gz"
REGEX='dbtl-v[0-9A-Za-z._-]+-x86_64-unknown-linux-gnu\.tar\.gz'

rewrite_file() {
  local path="$1"
  perl -0pi -e 's/'"$REGEX"'/'"$EXPECTED_ASSET"'/g' "$path"
}

if [[ "$MODE" == "write" ]]; then
  rewrite_file "$README_PATH"
  echo "Updated $README_PATH to use $EXPECTED_ASSET"
  exit 0
fi

TMP_FILE=$(mktemp)
cp "$README_PATH" "$TMP_FILE"
rewrite_file "$TMP_FILE"

if cmp -s "$README_PATH" "$TMP_FILE"; then
  echo "README install version is up to date for tag $LATEST_TAG"
  rm -f "$TMP_FILE"
  exit 0
fi

echo "README install version is stale; expected asset: $EXPECTED_ASSET" >&2
diff -u "$README_PATH" "$TMP_FILE" || true
rm -f "$TMP_FILE"
exit 1
