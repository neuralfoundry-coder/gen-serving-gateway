#!/bin/bash
set -e

# Docker Installation Script for Ubuntu 24.xx
# Run: curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/generative-img-serving/main/deploy/install-docker.sh | bash

echo "============================================"
echo "  Docker Installation for Ubuntu 24.xx"
echo "============================================"

# Check if running as root
if [[ $EUID -eq 0 ]]; then
    SUDO=""
else
    SUDO="sudo"
fi

# Check if Docker is already installed
if command -v docker &> /dev/null; then
    echo "[INFO] Docker is already installed: $(docker --version)"
    
    # Check if Docker service is running
    if $SUDO systemctl is-active --quiet docker; then
        echo "[INFO] Docker service is running"
    else
        echo "[INFO] Starting Docker service..."
        $SUDO systemctl start docker
        $SUDO systemctl enable docker
    fi
else
    echo "[INFO] Installing Docker..."
    
    # Install Docker using official script
    curl -fsSL https://get.docker.com | $SUDO sh
    
    # Start and enable Docker
    $SUDO systemctl start docker
    $SUDO systemctl enable docker
    
    echo "[INFO] Docker installed successfully"
fi

# Add current user to docker group (if not root)
if [[ $EUID -ne 0 ]]; then
    if ! groups $USER | grep -q docker; then
        echo "[INFO] Adding $USER to docker group..."
        $SUDO usermod -aG docker $USER
        echo "[WARN] Please log out and log back in for group changes to take effect"
        echo "[WARN] Or run: newgrp docker"
    fi
fi

# Install Docker Compose plugin (if not installed)
if ! docker compose version &> /dev/null; then
    echo "[INFO] Installing Docker Compose plugin..."
    $SUDO apt-get update
    $SUDO apt-get install -y docker-compose-plugin
fi

echo ""
echo "============================================"
echo "  Installation Complete!"
echo "============================================"
echo ""
echo "Docker version: $(docker --version)"
echo "Docker Compose version: $(docker compose version)"
echo ""
echo "Next steps:"
echo "  1. Log out and log back in (if added to docker group)"
echo "  2. Run deployment script: ./deploy-docker.sh"
echo ""

