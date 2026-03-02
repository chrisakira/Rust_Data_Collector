#!/usr/bin/env bash
set -euo pipefail

ENV_FILE=".env"

# Load .env if it exists
if [ -f "${ENV_FILE}" ]; then
  echo "📄 Loading environment from ${ENV_FILE}..."
  set -a
  source "${ENV_FILE}"
  set +a
else
  echo "❌ .env file not found. Please create one based on the README."
  exit 1
fi

echo "🚀 Running dollar-brl locally (without Docker Compose)..."

docker run --rm \
  --name dollar-brl-local \
  --network host \
  -e INFLUX_HOST="${INFLUX_HOST}" \
  -e INFLUX_TOKEN="${INFLUX_TOKEN}" \
  -e INFLUX_DATABASE="${INFLUX_DATABASE}" \
  dollar-brl:latest

echo "✅ Done!"