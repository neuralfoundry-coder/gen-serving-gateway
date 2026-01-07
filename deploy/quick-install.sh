#!/bin/bash
set -e

# Generative Image Serving - One-Line Quick Install Script
# 
# Supported OS: Ubuntu, Debian, CentOS, RHEL, Fedora, Amazon Linux, macOS
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/gen-serving-gateway/main/deploy/quick-install.sh | bash -s compose
#   curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/gen-serving-gateway/main/deploy/quick-install.sh | bash -s docker
#
# With options:
#   curl -fsSL .../quick-install.sh | HOST_PORT=9090 bash -s compose

REPO_URL="https://raw.githubusercontent.com/neuralfoundry-coder/gen-serving-gateway/main/deploy"
INSTALL_DIR="${INSTALL_DIR:-$HOME/gen-gateway}"
METHOD="${1:-compose}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${BLUE}[STEP]${NC} $1"; }

# ============================================
# OS Detection
# ============================================

detect_os() {
    OS=""
    OS_VERSION=""
    ARCH=$(uname -m)
    
    if [[ "$OSTYPE" == "darwin"* ]]; then
        OS="macos"
        OS_VERSION=$(sw_vers -productVersion)
    elif [[ -f /etc/os-release ]]; then
        . /etc/os-release
        OS="${ID}"
        OS_VERSION="${VERSION_ID}"
    elif [[ -f /etc/redhat-release ]]; then
        OS="centos"
    elif [[ -f /etc/debian_version ]]; then
        OS="debian"
    else
        OS="unknown"
    fi
    
    # Normalize OS names
    case "$OS" in
        ubuntu|debian|linuxmint|pop)
            OS_FAMILY="debian"
            ;;
        centos|rhel|rocky|almalinux|ol)
            OS_FAMILY="rhel"
            ;;
        fedora)
            OS_FAMILY="fedora"
            ;;
        amzn)
            OS_FAMILY="amazon"
            ;;
        macos|darwin)
            OS_FAMILY="macos"
            ;;
        *)
            OS_FAMILY="unknown"
            ;;
    esac
    
    log_info "Detected OS: $OS $OS_VERSION ($OS_FAMILY) - $ARCH"
}

# ============================================
# Privilege Management
# ============================================

acquire_privileges() {
    if [[ $EUID -eq 0 ]]; then
        SUDO=""
        log_info "Running as root"
        return 0
    fi
    
    log_step "Acquiring administrator privileges..."
    
    # Request sudo upfront and keep it alive
    if sudo -v; then
        SUDO="sudo"
        log_info "Administrator privileges acquired"
        
        # Keep sudo alive in background
        (while true; do sudo -n true; sleep 50; kill -0 "$$" || exit; done 2>/dev/null) &
        SUDO_KEEPALIVE_PID=$!
        
        # Cleanup on exit
        trap 'kill $SUDO_KEEPALIVE_PID 2>/dev/null' EXIT
    else
        log_error "Failed to acquire administrator privileges"
        exit 1
    fi
}

# ============================================
# Docker Installation by OS
# ============================================

check_docker_installed() {
    if command -v docker &> /dev/null; then
        DOCKER_VERSION=$(docker --version 2>/dev/null | cut -d' ' -f3 | tr -d ',')
        log_info "Docker is installed: v$DOCKER_VERSION"
        return 0
    fi
    return 1
}

check_docker_running() {
    if $SUDO docker info &> /dev/null; then
        return 0
    fi
    return 1
}

check_docker_compose() {
    if $SUDO docker compose version &> /dev/null; then
        COMPOSE_VERSION=$($SUDO docker compose version --short 2>/dev/null)
        log_info "Docker Compose is installed: v$COMPOSE_VERSION"
        return 0
    fi
    return 1
}

install_docker_debian() {
    log_step "Installing Docker on Debian/Ubuntu..."
    
    # Remove old versions
    $SUDO apt-get remove -y docker docker-engine docker.io containerd runc 2>/dev/null || true
    
    # Install prerequisites
    $SUDO apt-get update
    $SUDO apt-get install -y \
        ca-certificates \
        curl \
        gnupg \
        lsb-release
    
    # Add Docker GPG key
    $SUDO install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/$OS/gpg | $SUDO gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    $SUDO chmod a+r /etc/apt/keyrings/docker.gpg
    
    # Add Docker repository
    echo \
        "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/$OS \
        $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
        $SUDO tee /etc/apt/sources.list.d/docker.list > /dev/null
    
    # Install Docker
    $SUDO apt-get update
    $SUDO apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
    
    log_info "Docker installed successfully"
}

install_docker_rhel() {
    log_step "Installing Docker on RHEL/CentOS..."
    
    # Remove old versions
    $SUDO yum remove -y docker docker-client docker-client-latest docker-common \
        docker-latest docker-latest-logrotate docker-logrotate docker-engine 2>/dev/null || true
    
    # Install prerequisites
    $SUDO yum install -y yum-utils
    
    # Add Docker repository
    $SUDO yum-config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo
    
    # Install Docker
    $SUDO yum install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
    
    log_info "Docker installed successfully"
}

install_docker_fedora() {
    log_step "Installing Docker on Fedora..."
    
    # Remove old versions
    $SUDO dnf remove -y docker docker-client docker-client-latest docker-common \
        docker-latest docker-latest-logrotate docker-logrotate docker-selinux \
        docker-engine-selinux docker-engine 2>/dev/null || true
    
    # Install prerequisites
    $SUDO dnf install -y dnf-plugins-core
    
    # Add Docker repository
    $SUDO dnf config-manager --add-repo https://download.docker.com/linux/fedora/docker-ce.repo
    
    # Install Docker
    $SUDO dnf install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
    
    log_info "Docker installed successfully"
}

install_docker_amazon() {
    log_step "Installing Docker on Amazon Linux..."
    
    # Install Docker
    $SUDO yum install -y docker
    
    # Install Docker Compose
    $SUDO curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" \
        -o /usr/local/bin/docker-compose
    $SUDO chmod +x /usr/local/bin/docker-compose
    $SUDO ln -sf /usr/local/bin/docker-compose /usr/bin/docker-compose
    
    log_info "Docker installed successfully"
}

install_docker_macos() {
    log_step "Installing Docker on macOS..."
    
    # Check if Homebrew is installed
    if ! command -v brew &> /dev/null; then
        log_info "Installing Homebrew..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    fi
    
    # Install Docker Desktop via Homebrew
    if ! brew list --cask docker &> /dev/null; then
        brew install --cask docker
        log_info "Docker Desktop installed. Please start Docker Desktop manually."
        log_warn "After starting Docker Desktop, run this script again."
        exit 0
    fi
    
    log_info "Docker is installed"
}

install_docker() {
    case "$OS_FAMILY" in
        debian)
            install_docker_debian
            ;;
        rhel)
            install_docker_rhel
            ;;
        fedora)
            install_docker_fedora
            ;;
        amazon)
            install_docker_amazon
            ;;
        macos)
            install_docker_macos
            ;;
        *)
            log_error "Unsupported OS: $OS ($OS_FAMILY)"
            log_info "Please install Docker manually: https://docs.docker.com/engine/install/"
            exit 1
            ;;
    esac
}

# ============================================
# Docker Service Management
# ============================================

start_docker_service() {
    if [[ "$OS_FAMILY" == "macos" ]]; then
        # macOS: Check if Docker Desktop is running
        if ! check_docker_running; then
            log_warn "Docker Desktop is not running. Please start it manually."
            log_info "Opening Docker Desktop..."
            open -a Docker 2>/dev/null || true
            
            log_info "Waiting for Docker to start (max 60 seconds)..."
            local count=0
            while ! check_docker_running && [[ $count -lt 60 ]]; do
                sleep 2
                ((count+=2))
                echo -n "."
            done
            echo ""
            
            if ! check_docker_running; then
                log_error "Docker failed to start. Please start Docker Desktop and run this script again."
                exit 1
            fi
        fi
    else
        # Linux: Start and enable Docker service
        log_step "Starting Docker service..."
        $SUDO systemctl start docker 2>/dev/null || $SUDO service docker start 2>/dev/null || true
        $SUDO systemctl enable docker 2>/dev/null || true
        
        # Wait for Docker to be ready
        local count=0
        while ! check_docker_running && [[ $count -lt 30 ]]; do
            sleep 1
            ((count++))
        done
        
        if ! check_docker_running; then
            log_error "Docker service failed to start"
            exit 1
        fi
        
        log_info "Docker service started"
    fi
}

setup_docker_permissions() {
    if [[ "$OS_FAMILY" == "macos" ]]; then
        return 0
    fi
    
    if [[ $EUID -eq 0 ]]; then
        return 0
    fi
    
    # Add current user to docker group
    if ! groups $USER | grep -q docker; then
        log_step "Adding $USER to docker group..."
        $SUDO usermod -aG docker $USER
        
        # Apply group change for current session
        log_info "Applying docker group permissions..."
        
        # Use newgrp in a subshell to apply changes
        if [[ -n "$SUDO" ]]; then
            # Create a script to run with new group
            $SUDO chmod 666 /var/run/docker.sock 2>/dev/null || true
        fi
        
        log_info "Docker group permissions configured"
        log_warn "For permanent effect, please log out and log back in"
    fi
}

# ============================================
# Main Installation
# ============================================

show_banner() {
    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║   Gen Serving Gateway - Quick Install                  ║${NC}"
    echo -e "${CYAN}║   (Image + Text AI Model Serving)                      ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

download_scripts() {
    log_step "Downloading deployment scripts..."
    
    # Download setup-backends script for backend configuration
    curl -fsSL "https://raw.githubusercontent.com/neuralfoundry-coder/gen-serving-gateway/main/scripts/setup-backends.sh" -o setup-backends.sh
    chmod +x setup-backends.sh
    
    # Download sample config files
    mkdir -p config
    curl -fsSL "https://raw.githubusercontent.com/neuralfoundry-coder/gen-serving-gateway/main/config/backends.yaml" -o config/backends.yaml 2>/dev/null || true
    curl -fsSL "https://raw.githubusercontent.com/neuralfoundry-coder/gen-serving-gateway/main/config/gateway.yaml" -o config/gateway.yaml 2>/dev/null || true
    
    case "$METHOD" in
        docker|1)
            curl -fsSL "$REPO_URL/deploy-docker.sh" -o deploy-docker.sh
            chmod +x deploy-docker.sh
            ;;
        compose|2)
            curl -fsSL "$REPO_URL/deploy-compose.sh" -o deploy-compose.sh
            curl -fsSL "$REPO_URL/docker-compose.yml" -o docker-compose.yml
            chmod +x deploy-compose.sh
            ;;
    esac
    
    log_info "Scripts downloaded"
}

setup_backends_interactive() {
    log_step "Backend Configuration"
    echo ""
    echo "Do you want to configure AI model backends now?"
    echo "  [1] Yes, run interactive setup"
    echo "  [2] No, use default configuration"
    echo ""
    read -p "Enter choice [1-2]: " setup_choice
    
    case "$setup_choice" in
        1)
            ./setup-backends.sh
            ;;
        *)
            log_info "Skipping backend setup. You can run it later with:"
            echo "  cd $INSTALL_DIR && ./setup-backends.sh"
            ;;
    esac
}

deploy_service() {
    log_step "Deploying service..."
    
    case "$METHOD" in
        docker|1)
            ./deploy-docker.sh deploy
            ;;
        compose|2)
            ./deploy-compose.sh deploy
            ;;
    esac
}

# Cleanup existing installation
cleanup_existing() {
    local container_name="${CONTAINER_NAME:-gen-gateway}"
    
    log_step "Checking for existing installation..."
    
    # Stop and remove existing container if it exists
    if $SUDO docker ps -a --format '{{.Names}}' 2>/dev/null | grep -q "^${container_name}$"; then
        log_warn "Found existing container '$container_name'. Removing..."
        $SUDO docker stop "$container_name" 2>/dev/null || true
        $SUDO docker rm "$container_name" 2>/dev/null || true
        log_info "Existing container removed"
    fi
    
    # Clean up existing install directory if it exists and has old files
    if [[ -d "$INSTALL_DIR" ]]; then
        if [[ -f "$INSTALL_DIR/deploy-docker.sh" ]] || [[ -f "$INSTALL_DIR/deploy-compose.sh" ]]; then
            log_warn "Found existing installation at $INSTALL_DIR"
            
            # Try to stop services if compose is being used
            if [[ -f "$INSTALL_DIR/docker-compose.yml" ]]; then
                log_info "Stopping existing compose services..."
                (cd "$INSTALL_DIR" && $SUDO docker compose down 2>/dev/null) || true
            fi
            
            # Backup config if exists
            if [[ -d "$INSTALL_DIR/config" ]]; then
                log_info "Backing up existing configuration..."
                cp -r "$INSTALL_DIR/config" "/tmp/gen-gateway-config-backup-$(date +%Y%m%d%H%M%S)" 2>/dev/null || true
            fi
            
            log_info "Cleaning up old installation files..."
            rm -f "$INSTALL_DIR/deploy-docker.sh" 2>/dev/null || true
            rm -f "$INSTALL_DIR/deploy-compose.sh" 2>/dev/null || true
            rm -f "$INSTALL_DIR/docker-compose.yml" 2>/dev/null || true
            rm -f "$INSTALL_DIR/setup-backends.sh" 2>/dev/null || true
            rm -f "$INSTALL_DIR/.env" 2>/dev/null || true
        fi
    fi
    
    log_info "Cleanup complete"
}

main() {
    show_banner
    
    # Detect OS
    detect_os
    
    # Acquire privileges early
    if [[ "$OS_FAMILY" != "macos" ]]; then
        acquire_privileges
    fi
    
    # Check and install Docker
    if ! check_docker_installed; then
        log_warn "Docker is not installed"
        install_docker
    fi
    
    # Start Docker service
    start_docker_service
    
    # Setup permissions
    setup_docker_permissions
    
    # Verify Docker is working
    if ! check_docker_running; then
        log_error "Docker is not running properly"
        exit 1
    fi
    
    # Check Docker Compose (for compose method)
    if [[ "$METHOD" == "compose" ]] || [[ "$METHOD" == "2" ]]; then
        if ! check_docker_compose; then
            log_warn "Docker Compose plugin not found"
            if [[ "$OS_FAMILY" != "macos" ]]; then
                log_info "Installing Docker Compose plugin..."
                $SUDO apt-get install -y docker-compose-plugin 2>/dev/null || \
                $SUDO yum install -y docker-compose-plugin 2>/dev/null || \
                $SUDO dnf install -y docker-compose-plugin 2>/dev/null || true
            fi
        fi
    fi
    
    # Cleanup existing installation
    cleanup_existing
    
    # Create install directory
    log_step "Creating install directory: $INSTALL_DIR"
    mkdir -p "$INSTALL_DIR"
    cd "$INSTALL_DIR"
    
    # Download scripts and config
    download_scripts
    
    # Optional: Interactive backend setup
    if [[ -t 0 ]]; then  # Only if running interactively
        setup_backends_interactive
    fi
    
    # Deploy service
    deploy_service
    
    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║   Installation Complete!                               ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "Install directory: $INSTALL_DIR"
    echo "API Endpoint: http://localhost:${HOST_PORT:-15115}"
    echo ""
    echo "Configuration:"
    echo "  Backend config: $INSTALL_DIR/config/backends.yaml"
    echo "  Gateway config: $INSTALL_DIR/config/gateway.yaml"
    echo "  Setup tool:     $INSTALL_DIR/setup-backends.sh"
    echo ""
    echo "API Endpoints (OpenAI compatible):"
    echo "  POST /v1/images/generations    - Image generation"
    echo "  POST /v1/chat/completions      - Chat completion"
    echo "  POST /v1/completions           - Text completion"
    echo "  GET  /v1/models                - List models"
    echo ""
    echo "Commands:"
    if [[ "$METHOD" == "docker" ]] || [[ "$METHOD" == "1" ]]; then
        echo "  cd $INSTALL_DIR && ./deploy-docker.sh logs"
        echo "  cd $INSTALL_DIR && ./deploy-docker.sh status"
        echo "  cd $INSTALL_DIR && ./setup-backends.sh"
    else
        echo "  cd $INSTALL_DIR && ./deploy-compose.sh logs"
        echo "  cd $INSTALL_DIR && ./deploy-compose.sh status"
        echo "  cd $INSTALL_DIR && ./setup-backends.sh"
    fi
    echo ""
}

main "$@"
