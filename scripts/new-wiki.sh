#!/usr/bin/env bash
# Bootstrap a new design wiki from wiki-kit.
# Usage: ./scripts/new-wiki.sh SLUG "Wiki title" [TARGET_DIR] [--github URL]
set -euo pipefail

usage() {
  echo "Usage: $0 SLUG \"Wiki title\" [TARGET_DIR] [--github URL]" >&2
  echo "  SLUG        URL segment: /{slug}/wiki/" >&2
  echo "  TARGET_DIR  default: ../{slug}.design" >&2
  exit 1
}

[[ $# -lt 2 ]] && usage

SLUG="$1"
TITLE="$2"
shift 2

TARGET=""
GITHUB=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --github)
      [[ $# -lt 2 ]] && usage
      GITHUB="$2"
      shift 2
      ;;
    *)
      if [[ -z "$TARGET" ]]; then
        TARGET="$1"
        shift
      else
        usage
      fi
      ;;
  esac
done

if [[ -z "$TARGET" ]]; then
  TARGET="$(cd "$(dirname "$0")/../.." && pwd)/${SLUG}.design"
fi

if [[ ! "$SLUG" =~ ^[a-z0-9]([a-z0-9-]*[a-z0-9])?$ ]]; then
  echo "Invalid slug: $SLUG (use lowercase letters, digits, hyphens)" >&2
  exit 1
fi

KIT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$(mkdir -p "$(dirname "$TARGET")" && cd "$(dirname "$TARGET")" && pwd)/$(basename "$TARGET")"

if [[ -e "$TARGET" ]] && [[ -n "$(ls -A "$TARGET" 2>/dev/null || true)" ]]; then
  echo "Target not empty: $TARGET" >&2
  exit 1
fi

mkdir -p "$TARGET"

rsync -a \
  --exclude node_modules \
  --exclude .git \
  --exclude .vitepress/dist \
  --exclude .vitepress/cache \
  --exclude content.zip \
  "$KIT_ROOT/" "$TARGET/"

# .wiki.json
GITHUB_JSON="\"\""
if [[ -n "$GITHUB" ]]; then
  GITHUB_JSON="\"$GITHUB\""
fi

cat >"$TARGET/.wiki.json" <<EOF
{
  "slug": "$SLUG",
  "title": "$TITLE",
  "description": "Канонический design для $TITLE",
  "lang": "ru-RU",
  "github": $GITHUB_JSON
}
EOF

chmod +x "$TARGET/scripts/"*.sh 2>/dev/null || true
chmod +x "$TARGET/.githooks/pre-push" 2>/dev/null || true

if [[ ! -d "$TARGET/.git" ]]; then
  git -C "$TARGET" init -b master
fi

echo ""
echo "Created: $TARGET"
echo "  slug:  $SLUG"
echo "  base:  /${SLUG}/wiki/"
echo ""
echo "Next:"
echo "  cd \"$TARGET\""
echo "  npm install"
echo "  npm run docs:dev"
echo "  # edit content/, .vitepress/config.ts sidebar"
echo "  git remote add origin git@github.com:OWNER/${SLUG}.design.git"
echo "  git add -A && git commit -m \"init from wiki-kit\""
