#!/usr/bin/env bash
# Implanta o servidor do Lume na VPS: código em /root/lume, contêiner em 127.0.0.1:3020,
# site do Caddy em /etc/caddy/sites/lume.caddy. Não mexe nos outros sites.
set -euo pipefail
HOST="${1:-vps}"
cd "$(dirname "$0")/.."

ssh "$HOST" "mkdir -p /root/lume"
rsync -az --delete --exclude node_modules --exclude dist --exclude .env --exclude data ./server/ "$HOST:/root/lume/server/"
rsync -az --delete --exclude node_modules --exclude dist ./capture/ "$HOST:/root/lume/capture/"
rsync -az ./test-fixtures/ "$HOST:/root/lume/test-fixtures/"
rsync -az --relative ./src/components/DeskScene.vue ./src/lib/timeOfDay.ts "$HOST:/root/lume/"
rsync -az ./.dockerignore "$HOST:/root/lume/"

ssh "$HOST" 'set -e
  cd /root/lume/server
  if [ ! -f .env ]; then
    (umask 077; echo "INVITE_CODE=$(openssl rand -hex 12)" > .env)
    echo "Código de convite novo gerado em /root/lume/server/.env"
  fi
  docker compose up -d --build
  install -m 644 -o root -g root lume.caddy /etc/caddy/sites/lume.caddy
  caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile
  systemctl reload caddy
  sleep 3
  curl -fsS http://127.0.0.1:3020/api/health && echo " <- contêiner ok"'
