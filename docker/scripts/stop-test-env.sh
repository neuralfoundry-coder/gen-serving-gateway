#!/bin/bash
# Stop the test environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$(dirname "$SCRIPT_DIR")"

echo "🛑 Stopping test environment..."

cd "$DOCKER_DIR"

# Stop and remove containers
docker-compose down -v

echo "✅ Test environment stopped"

