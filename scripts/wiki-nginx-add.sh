#!/usr/bin/env bash
# Add or update one wiki location on the VPS nginx hub.
# Usage: ./scripts/wiki-nginx-add.sh SLUG [WIKI_ROOT]
set -euo pipefail

usage() {
  echo "Usage: $0 SLUG [WIKI_ROOT]" >&2
  echo "  WIKI_ROOT default: \$HOME/{slug}-wiki/www" >&2
  exit 1
}

[[ $# -lt 1 ]] && usage

SLUG="$1"
WIKI_ROOT="${2:-$HOME/${SLUG}-wiki/www}"
WIKI_LOCATIONS_DIR="${WIKI_LOCATIONS_DIR:-/etc/nginx/wiki-locations}"
WG_HUB_IP="${WG_HUB_IP:-10.243.63.1}"

if [[ ! "$SLUG" =~ ^[a-z0-9]([a-z0-9-]*[a-z0-9])?$ ]]; then
  echo "Invalid slug: $SLUG" >&2
  exit 1
fi

mkdir -p "$WIKI_ROOT"
chmod -R a+rX "$(dirname "$WIKI_ROOT")" "$WIKI_ROOT" 2>/dev/null || true

CONF="${WIKI_LOCATIONS_DIR}/${SLUG}.conf"

sudo mkdir -p "$WIKI_LOCATIONS_DIR"

sudo tee "$CONF" >/dev/null <<EOF
# wiki: ${SLUG}
location /${SLUG}/wiki/ {
    alias ${WIKI_ROOT}/;
    index index.html;
    try_files \$uri \$uri.html \$uri/ =404;
    add_header X-Robots-Tag "noindex, nofollow, noarchive, nosnippet" always;
}

location = /${SLUG}/wiki {
    return 301 /${SLUG}/wiki/;
}
EOF

if ! command -v nginx >/dev/null 2>&1; then
  echo "nginx not installed. Run setup-vps-wiki-nginx.sh first." >&2
  exit 1
fi

sudo nginx -t
sudo systemctl reload nginx

echo "OK: http://${WG_HUB_IP}/${SLUG}/wiki/"
echo "     root: ${WIKI_ROOT}"
