#!/bin/bash
# Start the test environment with Docker Compose

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_DIR="$(dirname "$DOCKER_DIR")"

echo "🚀 Starting test environment..."

cd "$DOCKER_DIR"

# Build and start services
docker-compose up -d --build

echo "⏳ Waiting for services to be healthy..."

# Wait for services to be ready
max_attempts=30
attempt=0

while [ $attempt -lt $max_attempts ]; do
    # Check if all services are healthy
    healthy_count=$(docker-compose ps | grep -c "(healthy)" || true)
    total_services=4  # http-mock-1, http-mock-2, slow-mock, failure-mock
    
    if [ "$healthy_count" -ge "$total_services" ]; then
        echo "✅ All services are healthy!"
        break
    fi
    
    attempt=$((attempt + 1))
    echo "   Waiting... ($attempt/$max_attempts)"
    sleep 2
done

if [ $attempt -eq $max_attempts ]; then
    echo "⚠️  Warning: Not all services became healthy in time"
    docker-compose ps
fi

echo ""
echo "📊 Service Status:"
docker-compose ps

echo ""
echo "🔗 Available endpoints:"
echo "   Gateway:        http://localhost:8080"
echo "   HTTP Mock 1:    http://localhost:7860"
echo "   HTTP Mock 2:    http://localhost:7861"
echo "   Slow Mock:      http://localhost:7862"
echo "   Failure Mock:   http://localhost:7863"
echo "   gRPC Mock:      localhost:50051"
echo ""
echo "📝 To view logs: docker-compose -f $DOCKER_DIR/docker-compose.yml logs -f"
echo "🛑 To stop: $SCRIPT_DIR/stop-test-env.sh"

