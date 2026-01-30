#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting FerroTunnel Manual Publish Process...${NC}"

# Ensure we are in the root directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must run from project root directory${NC}"
    exit 1
fi

# Get version from Cargo.toml
VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
echo -e "Publishing version: ${GREEN}${VERSION}${NC}"

# Check for uncommitted changes
if [ -n "$(git status --porcelain)" ]; then
    echo -e "${RED}Error: Working directory is not clean. Please commit changes first.${NC}"
    exit 1
fi

# Run checks
echo -e "\n${GREEN}Running checks...${NC}"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Function to publish and wait
publish_crate() {
    CRATE_NAME=$1
    echo -e "\n${GREEN}Publishing ${CRATE_NAME}...${NC}"

    cargo publish -p "${CRATE_NAME}"

    echo "Waiting for ${CRATE_NAME} v${VERSION} to verify on crates.io..."
    for i in {1..30}; do
        if curl -s -f "https://crates.io/api/v1/crates/${CRATE_NAME}/${VERSION}" > /dev/null; then
            echo -e "${GREEN}${CRATE_NAME} is available!${NC}"
            return 0
        fi
        echo -n "."
        sleep 10
    done

    echo -e "\n${RED}Timeout waiting for ${CRATE_NAME} to appear on crates.io${NC}"
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
}

# Publish in dependency order
publish_crate "ferrotunnel-common"
publish_crate "ferrotunnel-protocol"
publish_crate "ferrotunnel-core"
publish_crate "ferrotunnel-plugin"
publish_crate "ferrotunnel-http"
publish_crate "ferrotunnel"
publish_crate "ferrotunnel-client"
publish_crate "ferrotunnel-server"

echo -e "\n${GREEN}All crates published successfully!${NC}"
