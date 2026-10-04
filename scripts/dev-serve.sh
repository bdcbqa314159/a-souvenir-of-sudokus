#!/usr/bin/env bash
# Dev server over HTTPS: https://localhost:8642
# Cert is self-signed and gitignored (.dev-certs/). With mkcert installed the
# cert is locally trusted (no browser warning); plain openssl otherwise —
# the browser then warns once: Advanced -> Proceed. Dev-only either way.
set -euo pipefail
cd "$(dirname "$0")/.."
CERTS=.dev-certs
mkdir -p "$CERTS"

if [ ! -f "$CERTS/localhost.pem" ]; then
  if command -v mkcert >/dev/null; then
    mkcert -install >/dev/null 2>&1 || true
    mkcert -cert-file "$CERTS/localhost.pem" -key-file "$CERTS/localhost-key.pem" localhost 127.0.0.1
  else
    openssl req -x509 -newkey rsa:2048 -nodes -days 825 \
      -keyout "$CERTS/localhost-key.pem" -out "$CERTS/localhost.pem" \
      -subj "/CN=localhost" \
      -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
  fi
fi

cd web
exec trunk serve --port 8642 \
  --tls-cert-path "../$CERTS/localhost.pem" \
  --tls-key-path "../$CERTS/localhost-key.pem"
