#!/usr/bin/env bash
set -euo pipefail

IMAGE_NAME="dollar-brl"
IMAGE_TAG="latest"

echo "🔨 Building Docker image: ${IMAGE_NAME}:${IMAGE_TAG}..."

docker build \
  --tag "${IMAGE_NAME}:${IMAGE_TAG}" \
  --file Dockerfile \
  .

echo "✅ Build complete: ${IMAGE_NAME}:${IMAGE_TAG}"