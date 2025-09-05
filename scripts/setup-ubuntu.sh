#!/usr/bin/env bash
set -e

echo "[1/3] Setting up test-env (Ubuntu 24.04.2 LTS)"

if ! command -v docker &> /dev/null; then
    echo "✖ Docker not found. Install Docker Desktop first."
    echo "   Download: https://www.docker.com/products/docker-desktop"
    exit 1
fi

if ! docker info &> /dev/null; then
    echo "✖ Docker is not running. Start Docker Desktop to continue."
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "✖ docker-compose not found. Ensure Docker Desktop is running."
    exit 1
fi

if [ ! -d "vendor/.git" ] || [ -z "$(ls -A vendor)" ]; then
    echo "[2/3] Initializing submodules"
    make vinit
fi

echo "[3/3] Building container (Ubuntu 24.04.2 LTS)"
make docker-build

echo ""
echo "√ Setup complete."
echo ""
echo "  - Tests: make docker-test"
echo "  - Shell: make docker-shell"  
echo "  - CI: make docker-ci"
echo ""
echo "  - Help: make help"
echo "  - Docs: cat TESTING.md"
echo ""
