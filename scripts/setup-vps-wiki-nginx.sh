#!/usr/bin/env bash
# One-time VPS setup: nginx wiki hub on WireGuard IP. Does not touch bot, wg, xray.
# Adds include dir for per-wiki location snippets (wiki-nginx-add.sh).
set -euo pipefail

WIKI_LOCATIONS_DIR="${WIKI_LOCATIONS_DIR:-/etc/nginx/wiki-locations}"
WG_HUB_IP="${WG_HUB_IP:-10.243.63.1}"
NGINX_SITE="/etc/nginx/sites-available/wiki-hub"
NGINX_BOT_MAP="/etc/nginx/conf.d/wiki-bot-block-map.conf"

echo "Wiki locations: $WIKI_LOCATIONS_DIR"
echo "Listen: ${WG_HUB_IP}:80 (WireGuard only)"

if ! command -v nginx >/dev/null 2>&1; then
  sudo apt-get update -qq
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y nginx
fi

sudo mkdir -p "$WIKI_LOCATIONS_DIR"

sudo tee "$NGINX_BOT_MAP" >/dev/null <<'EOF'
map $http_user_agent $wiki_block_bot {
    default 0;
    "" 1;
    "-" 1;
    ~*googlebot 1;
    ~*bingbot 1;
    ~*yandex 1;
    ~*baiduspider 1;
    ~*duckduckbot 1;
    ~*slurp 1;
    ~*facebookexternalhit 1;
    ~*twitterbot 1;
    ~*linkedinbot 1;
    ~*embedly 1;
    ~*pinterest 1;
    ~*applebot 1;
    ~*petalbot 1;
    ~*semrush 1;
    ~*ahrefs 1;
    ~*dotbot 1;
    ~*rogerbot 1;
    ~*archive.org_bot 1;
    ~*wget 1;
    ~*python-requests 1;
    ~*go-http-client 1;
    ~*scrapy 1;
    ~*claudebot 1;
    ~*claude-user 1;
    ~*gptbot 1;
    ~*chatgpt-user 1;
    ~*anthropic 1;
    ~*bot 1;
    ~*crawl 1;
    ~*spider 1;
}
EOF

sudo tee "$NGINX_SITE" >/dev/null <<EOF
server {
    listen ${WG_HUB_IP}:80;
    listen 127.0.0.1:80;

    if (\$wiki_block_bot) {
        return 403;
    }

    location = /robots.txt {
        default_type text/plain;
        return 200 "User-agent: *\\nDisallow: /\\n";
    }

    location = / {
        return 404;
    }

    include ${WIKI_LOCATIONS_DIR}/*.conf;

    location / {
        return 404;
    }
}
EOF

sudo ln -sf "$NGINX_SITE" /etc/nginx/sites-enabled/wiki-hub
sudo rm -f /etc/nginx/sites-enabled/default

# Legacy single-wiki site from utlas.design (disable if migrating)
if [[ -f /etc/nginx/sites-enabled/utlas-wiki ]]; then
  echo "Note: disabling legacy sites-enabled/utlas-wiki — re-add utlas via wiki-nginx-add.sh"
  sudo rm -f /etc/nginx/sites-enabled/utlas-wiki
fi

sudo nginx -t
sudo systemctl enable nginx
sudo systemctl reload nginx

echo "OK. Add wikis: bash scripts/wiki-nginx-add.sh {slug}"
