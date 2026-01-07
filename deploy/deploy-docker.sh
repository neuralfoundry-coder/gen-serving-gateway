#!/bin/bash
set -e

# Generative Image Serving - Docker Deployment Script (Method 1)
# Direct Docker run without Docker Compose

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Set sudo if not root and docker requires it
SUDO=""
if [[ $EUID -ne 0 ]]; then
    if ! docker info &> /dev/null 2>&1; then
        SUDO="sudo"
    fi
fi

# Configuration
IMAGE_NAME="neuralfoundry2coder/gen-serving-gateway"
IMAGE_TAG="${IMAGE_TAG:-latest}"
CONTAINER_NAME="${CONTAINER_NAME:-gen-gateway}"
HOST_PORT="${HOST_PORT:-15115}"
CONTAINER_PORT="15115"
DATA_DIR="${DATA_DIR:-$SCRIPT_DIR/data}"
CONFIG_DIR="${DATA_DIR}/config"
IMAGES_DIR="${DATA_DIR}/generated_images"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${BLUE}[STEP]${NC} $1"; }

show_banner() {
    echo ""
    echo -e "${BLUE}============================================${NC}"
    echo -e "${BLUE}  Gen Serving Gateway - Docker Deploy${NC}"
    echo -e "${BLUE}  (Image + Text AI Model Serving)${NC}"
    echo -e "${BLUE}============================================${NC}"
    echo ""
}

check_docker() {
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed. Run install-docker.sh first."
        exit 1
    fi
    
    if ! $SUDO docker info &> /dev/null; then
        log_error "Docker daemon is not running or you don't have permission."
        log_info "Try: sudo systemctl start docker"
        log_info "Or:  newgrp docker"
        exit 1
    fi
}

create_directories() {
    log_step "Creating data directories..."
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$IMAGES_DIR"
    log_info "Data directory: $DATA_DIR"
}

create_default_config() {
    local config_file="$CONFIG_DIR/default.toml"
    
    if [[ -f "$config_file" ]]; then
        log_info "Config file already exists: $config_file"
        return
    fi
    
    log_step "Creating default configuration..."
    cat > "$config_file" << 'EOF'
[server]
host = "0.0.0.0"
port = 15115

[auth]
enabled = false
api_keys = []

[rate_limit]
enabled = true
requests_per_second = 100
burst_size = 200

[storage]
path = "/app/generated_images"
max_age_hours = 24
cleanup_interval_secs = 3600

[load_balancer]
strategy = "round_robin"

# Example backend configuration
# Uncomment and modify to add your image generation backends
#
# [[backends]]
# name = "stable-diffusion"
# protocol = "http"
# endpoints = ["http://localhost:8001"]
# health_check_path = "/health"
# health_check_interval_secs = 30
# timeout_ms = 60000
# weight = 1
# enabled = true
EOF
    log_info "Config created: $config_file"
}

pull_image() {
    log_step "Pulling Docker image: $IMAGE_NAME:$IMAGE_TAG"
    $SUDO docker pull "$IMAGE_NAME:$IMAGE_TAG"
    log_info "Image pulled successfully"
}

stop_existing() {
    if $SUDO docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        log_step "Stopping existing container..."
        $SUDO docker stop "$CONTAINER_NAME" 2>/dev/null || true
        $SUDO docker rm "$CONTAINER_NAME" 2>/dev/null || true
        log_info "Existing container removed"
    fi
}

run_container() {
    log_step "Starting container..."
    
    $SUDO docker run -d \
        --name "$CONTAINER_NAME" \
        -p "${HOST_PORT}:${CONTAINER_PORT}" \
        -v "${CONFIG_DIR}:/app/config:ro" \
        -v "${IMAGES_DIR}:/app/generated_images" \
        -e RUST_LOG="${RUST_LOG:-info}" \
        --restart unless-stopped \
        --health-cmd="curl -f http://localhost:15115/health || exit 1" \
        --health-interval=30s \
        --health-timeout=10s \
        --health-retries=3 \
        --health-start-period=10s \
        "$IMAGE_NAME:$IMAGE_TAG"
    
    log_info "Container started: $CONTAINER_NAME"
}

wait_for_healthy() {
    log_step "Waiting for service to be healthy..."
    local max_attempts=30
    local attempt=1
    
    while [[ $attempt -le $max_attempts ]]; do
        if curl -sf "http://localhost:${HOST_PORT}/health" > /dev/null 2>&1; then
            log_info "Service is healthy!"
            return 0
        fi
        echo -n "."
        sleep 1
        ((attempt++))
    done
    
    echo ""
    log_warn "Service may not be fully ready. Check logs with: docker logs $CONTAINER_NAME"
}

show_status() {
    echo ""
    echo -e "${GREEN}============================================${NC}"
    echo -e "${GREEN}  Deployment Complete!${NC}"
    echo -e "${GREEN}============================================${NC}"
    echo ""
    echo "Container: $CONTAINER_NAME"
    echo "Image:     $IMAGE_NAME:$IMAGE_TAG"
    echo "Port:      $HOST_PORT"
    echo "Config:    $CONFIG_DIR"
    echo "Images:    $IMAGES_DIR"
    echo ""
    echo "Health Check:"
    curl -s "http://localhost:${HOST_PORT}/health" 2>/dev/null | head -c 200 || echo "  (waiting...)"
    echo ""
    echo ""
    echo "API Endpoints (OpenAI compatible):"
    echo "  POST /v1/images/generations    - Image generation"
    echo "  POST /v1/chat/completions      - Chat completion"
    echo "  POST /v1/completions           - Text completion"
    echo "  GET  /v1/models                - List models"
    echo ""
    echo "Commands:"
    echo "  View logs:    docker logs -f $CONTAINER_NAME"
    echo "  Stop:         docker stop $CONTAINER_NAME"
    echo "  Start:        docker start $CONTAINER_NAME"
    echo "  Restart:      docker restart $CONTAINER_NAME"
    echo "  Remove:       docker rm -f $CONTAINER_NAME"
    echo ""
    echo "Base URL: http://localhost:${HOST_PORT}"
    echo ""
}

usage() {
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo ""
    echo "Commands:"
    echo "  deploy    Deploy/update the service (default)"
    echo "  stop      Stop the service"
    echo "  start     Start the service"
    echo "  restart   Restart the service"
    echo "  logs      View logs"
    echo "  status    Show status"
    echo "  remove    Remove the container"
    echo ""
    echo "Environment Variables:"
    echo "  IMAGE_TAG       Image tag (default: latest)"
    echo "  CONTAINER_NAME  Container name (default: gen-gateway)"
    echo "  HOST_PORT       Host port (default: 15115)"
    echo "  DATA_DIR        Data directory (default: ./data)"
    echo "  RUST_LOG        Log level (default: info)"
    echo ""
    echo "Examples:"
    echo "  $0 deploy"
    echo "  HOST_PORT=9090 $0 deploy"
    echo "  IMAGE_TAG=0.2.0 $0 deploy"
}

main() {
    local command="${1:-deploy}"
    
    case "$command" in
        deploy)
            show_banner
            check_docker
            create_directories
            create_default_config
            pull_image
            stop_existing
            run_container
            wait_for_healthy
            show_status
            ;;
        stop)
            $SUDO docker stop "$CONTAINER_NAME"
            log_info "Container stopped"
            ;;
        start)
            $SUDO docker start "$CONTAINER_NAME"
            log_info "Container started"
            ;;
        restart)
            $SUDO docker restart "$CONTAINER_NAME"
            log_info "Container restarted"
            ;;
        logs)
            $SUDO docker logs -f "$CONTAINER_NAME"
            ;;
        status)
            $SUDO docker ps -a --filter "name=$CONTAINER_NAME"
            echo ""
            curl -s "http://localhost:${HOST_PORT}/health" 2>/dev/null || echo "Service not responding"
            ;;
        remove)
            $SUDO docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
            log_info "Container removed"
            ;;
        -h|--help|help)
            usage
            ;;
        *)
            log_error "Unknown command: $command"
            usage
            exit 1
            ;;
    esac
}

main "$@"

