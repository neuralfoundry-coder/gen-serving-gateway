#!/bin/bash
set -e

# Docker Hub Push Script
# This script builds and pushes the Docker image to Docker Hub

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Load environment variables from .env file
load_env() {
    local env_file="$PROJECT_ROOT/.env"
    
    if [[ -f "$env_file" ]]; then
        log_info "Loading environment from .env file..."
        set -a
        source "$env_file"
        set +a
    else
        log_error ".env file not found at $env_file"
        log_info "Please copy .env.example to .env and fill in your Docker Hub credentials"
        exit 1
    fi
}

# Validate required environment variables
validate_env() {
    local missing=()
    
    # Support both naming conventions
    DOCKER_USERNAME="${DOCKER_USERNAME:-$DOCKER_HUB_USERNAME}"
    DOCKER_ACCESS_TOKEN="${DOCKER_ACCESS_TOKEN:-$DOCKER_HUB_TOKEN}"
    IMAGE_NAME="${IMAGE_NAME:-generative-img-serving}"
    
    [[ -z "$DOCKER_USERNAME" ]] && missing+=("DOCKER_USERNAME or DOCKER_HUB_USERNAME")
    [[ -z "$DOCKER_ACCESS_TOKEN" ]] && missing+=("DOCKER_ACCESS_TOKEN or DOCKER_HUB_TOKEN")
    
    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required environment variables: ${missing[*]}"
        exit 1
    fi
}

# Get version from Cargo.toml
get_version() {
    grep '^version' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/.*= *"\(.*\)"/\1/'
}

# Login to Docker Hub
docker_login() {
    log_info "Logging in to Docker Hub..."
    echo "$DOCKER_ACCESS_TOKEN" | docker login -u "$DOCKER_USERNAME" --password-stdin
}

# Build Docker image
build_image() {
    local full_image_name="$DOCKER_USERNAME/$IMAGE_NAME"
    local version="$1"
    
    log_info "Building Docker image: $full_image_name:$version"
    
    docker build \
        --platform linux/amd64 \
        -t "$full_image_name:$version" \
        -t "$full_image_name:latest" \
        -f "$PROJECT_ROOT/Dockerfile" \
        "$PROJECT_ROOT"
    
    log_info "Image built successfully"
}

# Push Docker image
push_image() {
    local full_image_name="$DOCKER_USERNAME/$IMAGE_NAME"
    local version="$1"
    
    log_info "Pushing $full_image_name:$version to Docker Hub..."
    docker push "$full_image_name:$version"
    
    log_info "Pushing $full_image_name:latest to Docker Hub..."
    docker push "$full_image_name:latest"
    
    log_info "Push completed successfully!"
    echo ""
    log_info "Image available at: https://hub.docker.com/r/$DOCKER_USERNAME/$IMAGE_NAME"
}

# Cleanup
cleanup() {
    log_info "Logging out from Docker Hub..."
    docker logout 2>/dev/null || true
}

# Show usage
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -v, --version VERSION   Override version tag (default: from Cargo.toml)"
    echo "  -b, --build-only        Build image without pushing"
    echo "  -p, --push-only         Push existing image without building"
    echo "  -h, --help              Show this help message"
    echo ""
    echo "Environment variables (set in .env file):"
    echo "  DOCKER_USERNAME         Docker Hub username"
    echo "  DOCKER_ACCESS_TOKEN     Docker Hub access token"
    echo "  IMAGE_NAME              Image name (default: generative-img-serving)"
}

# Main
main() {
    local version=""
    local build_only=false
    local push_only=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -v|--version)
                version="$2"
                shift 2
                ;;
            -b|--build-only)
                build_only=true
                shift
                ;;
            -p|--push-only)
                push_only=true
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
    
    # Load and validate environment
    load_env
    validate_env
    
    # Get version if not specified
    if [[ -z "$version" ]]; then
        version=$(get_version)
    fi
    
    log_info "Image: $DOCKER_USERNAME/$IMAGE_NAME"
    log_info "Version: $version"
    echo ""
    
    # Set trap for cleanup
    trap cleanup EXIT
    
    # Login to Docker Hub
    docker_login
    
    # Build and/or push
    if [[ "$push_only" != true ]]; then
        build_image "$version"
    fi
    
    if [[ "$build_only" != true ]]; then
        push_image "$version"
    fi
    
    log_info "Done!"
}

main "$@"

