#!/bin/bash
set -e

# Generative Image Serving - Docker Compose Deployment Script (Method 2)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Set sudo if not root and docker requires it
SUDO=""
if [[ $EUID -ne 0 ]]; then
    if ! docker info &> /dev/null 2>&1; then
        SUDO="sudo"
    fi
fi

# Configuration
export IMAGE_TAG="${IMAGE_TAG:-latest}"
export CONTAINER_NAME="${CONTAINER_NAME:-gen-gateway}"
export HOST_PORT="${HOST_PORT:-15115}"
export RUST_LOG="${RUST_LOG:-info}"

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
    echo -e "${BLUE}  Generative Image Serving - Compose Deploy${NC}"
    echo -e "${BLUE}============================================${NC}"
    echo ""
}

check_docker() {
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed. Run install-docker.sh first."
        exit 1
    fi
    
    if ! $SUDO docker compose version &> /dev/null; then
        log_error "Docker Compose is not installed."
        exit 1
    fi
    
    if ! $SUDO docker info &> /dev/null; then
        log_error "Docker daemon is not running."
        exit 1
    fi
}

create_directories() {
    log_step "Creating directories..."
    mkdir -p config
    mkdir -p generated_images
}

create_default_config() {
    local config_file="config/default.toml"
    
    if [[ -f "$config_file" ]]; then
        log_info "Config file already exists"
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
# [[backends]]
# name = "stable-diffusion"
# protocol = "http"
# endpoints = ["http://localhost:8001"]
# weight = 1
# enabled = true
EOF
    log_info "Config created: $config_file"
}

create_env_file() {
    if [[ ! -f ".env" ]]; then
        log_step "Creating .env file..."
        cat > .env << EOF
# Docker Compose Environment Variables
IMAGE_TAG=${IMAGE_TAG}
CONTAINER_NAME=${CONTAINER_NAME}
HOST_PORT=${HOST_PORT}
RUST_LOG=${RUST_LOG}
EOF
        log_info ".env file created"
    fi
}

deploy() {
    show_banner
    check_docker
    create_directories
    create_default_config
    create_env_file
    
    log_step "Pulling latest image..."
    $SUDO docker compose pull
    
    log_step "Starting services..."
    $SUDO docker compose up -d
    
    log_step "Waiting for service to be healthy..."
    local max_attempts=30
    local attempt=1
    
    while [[ $attempt -le $max_attempts ]]; do
        if curl -sf "http://localhost:${HOST_PORT}/health" > /dev/null 2>&1; then
            break
        fi
        echo -n "."
        sleep 1
        ((attempt++))
    done
    echo ""
    
    show_status
}

show_status() {
    echo ""
    echo -e "${GREEN}============================================${NC}"
    echo -e "${GREEN}  Deployment Complete!${NC}"
    echo -e "${GREEN}============================================${NC}"
    echo ""
    $SUDO docker compose ps
    echo ""
    echo "Health Check:"
    curl -s "http://localhost:${HOST_PORT}/health" 2>/dev/null || echo "  (waiting...)"
    echo ""
    echo ""
    echo "Commands:"
    echo "  View logs:    docker compose logs -f"
    echo "  Stop:         docker compose stop"
    echo "  Start:        docker compose start"
    echo "  Restart:      docker compose restart"
    echo "  Remove:       docker compose down"
    echo "  Update:       docker compose pull && docker compose up -d"
    echo ""
    echo "API Endpoint: http://localhost:${HOST_PORT}"
    echo ""
}

usage() {
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  deploy    Deploy/update the service (default)"
    echo "  up        Start services"
    echo "  down      Stop and remove services"
    echo "  stop      Stop services"
    echo "  start     Start services"
    echo "  restart   Restart services"
    echo "  logs      View logs"
    echo "  status    Show status"
    echo "  pull      Pull latest image"
    echo "  update    Pull and restart"
    echo ""
    echo "Environment Variables:"
    echo "  IMAGE_TAG       Image tag (default: latest)"
    echo "  CONTAINER_NAME  Container name (default: gen-gateway)"
    echo "  HOST_PORT       Host port (default: 15115)"
    echo "  RUST_LOG        Log level (default: info)"
    echo ""
    echo "Examples:"
    echo "  $0 deploy"
    echo "  $0 logs"
    echo "  HOST_PORT=9090 $0 deploy"
}

main() {
    local command="${1:-deploy}"
    
    case "$command" in
        deploy)
            deploy
            ;;
        up)
            $SUDO docker compose up -d
            ;;
        down)
            $SUDO docker compose down
            log_info "Services stopped and removed"
            ;;
        stop)
            $SUDO docker compose stop
            log_info "Services stopped"
            ;;
        start)
            $SUDO docker compose start
            log_info "Services started"
            ;;
        restart)
            $SUDO docker compose restart
            log_info "Services restarted"
            ;;
        logs)
            $SUDO docker compose logs -f
            ;;
        status)
            $SUDO docker compose ps
            echo ""
            curl -s "http://localhost:${HOST_PORT}/health" 2>/dev/null || echo "Service not responding"
            ;;
        pull)
            $SUDO docker compose pull
            log_info "Image pulled"
            ;;
        update)
            log_step "Updating service..."
            $SUDO docker compose pull
            $SUDO docker compose up -d
            log_info "Service updated"
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

