#!/bin/bash
set -e

# Generative Image Serving - One-Line Quick Install Script
# 
# Method 1 (Docker):
#   curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/generative-img-serving/main/deploy/quick-install.sh | bash -s docker
#
# Method 2 (Docker Compose):
#   curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/generative-img-serving/main/deploy/quick-install.sh | bash -s compose
#
# With custom port:
#   curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/generative-img-serving/main/deploy/quick-install.sh | HOST_PORT=9090 bash -s compose

REPO_URL="https://raw.githubusercontent.com/neuralfoundry-coder/generative-img-serving/main/deploy"
INSTALL_DIR="${INSTALL_DIR:-$HOME/img-serving}"
METHOD="${1:-compose}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

echo ""
echo -e "${BLUE}============================================${NC}"
echo -e "${BLUE}  Generative Image Serving - Quick Install${NC}"
echo -e "${BLUE}============================================${NC}"
echo ""

# Create install directory
log_info "Creating install directory: $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"
cd "$INSTALL_DIR"

# Download install-docker.sh
log_info "Downloading Docker installer..."
curl -fsSL "$REPO_URL/install-docker.sh" -o install-docker.sh
chmod +x install-docker.sh

# Install Docker if needed
if ! command -v docker &> /dev/null; then
    log_info "Installing Docker..."
    ./install-docker.sh
fi

case "$METHOD" in
    docker|1)
        log_info "Setting up Method 1: Docker direct..."
        curl -fsSL "$REPO_URL/deploy-docker.sh" -o deploy-docker.sh
        chmod +x deploy-docker.sh
        
        log_info "Deploying..."
        ./deploy-docker.sh deploy
        ;;
    
    compose|2)
        log_info "Setting up Method 2: Docker Compose..."
        curl -fsSL "$REPO_URL/deploy-compose.sh" -o deploy-compose.sh
        curl -fsSL "$REPO_URL/docker-compose.yml" -o docker-compose.yml
        chmod +x deploy-compose.sh
        
        log_info "Deploying..."
        ./deploy-compose.sh deploy
        ;;
    
    *)
        log_error "Unknown method: $METHOD"
        echo "Usage: $0 [docker|compose]"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}============================================${NC}"
echo -e "${GREEN}  Installation Complete!${NC}"
echo -e "${GREEN}============================================${NC}"
echo ""
echo "Install directory: $INSTALL_DIR"
echo "API Endpoint: http://localhost:${HOST_PORT:-8080}"
echo ""
echo "Next steps:"
echo "  cd $INSTALL_DIR"
if [[ "$METHOD" == "docker" ]] || [[ "$METHOD" == "1" ]]; then
    echo "  ./deploy-docker.sh logs    # View logs"
    echo "  ./deploy-docker.sh status  # Check status"
else
    echo "  ./deploy-compose.sh logs   # View logs"
    echo "  ./deploy-compose.sh status # Check status"
fi
echo ""

