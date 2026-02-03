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

# Function to check if crate version already exists
check_exists() {
    CRATE_NAME=$1
    # crates.io API requires a User-Agent
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "User-Agent: FerroTunnel-Publish-Script (https://github.com/MitulShah1/ferrotunnel)" \
        "https://crates.io/api/v1/crates/${CRATE_NAME}/${VERSION}")

    if [ "$HTTP_CODE" = "200" ]; then
        return 0 # Exists
    fi
    return 1 # Does not exist or error
}

# Function to publish a single crate
publish_crate() {
    CRATE_NAME=$1

    if check_exists "${CRATE_NAME}"; then
        echo -e "${GREEN}${CRATE_NAME} v${VERSION} already exists on crates.io, skipping.${NC}"
        return 0
    fi

    echo -e "Publishing ${CRATE_NAME}..."
    # Capture output to check for "already exists" error if API check failed/lagged
    OUTPUT=$(cargo publish -p "${CRATE_NAME}" --no-verify 2>&1)
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}Successfully initiated publish for ${CRATE_NAME}. Sleeping 10s for indexing...${NC}"
        sleep 10
        return 0
    elif echo "$OUTPUT" | grep -q "already exists"; then
        echo -e "${GREEN}${CRATE_NAME} v${VERSION} already exists (caught by cargo), skipping.${NC}"
        return 0
    else
        echo -e "${RED}Failed to publish ${CRATE_NAME}${NC}"
        echo "$OUTPUT"
        return 1
    fi
}

# Function to run multiple publishes in parallel and wait for all
run_parallel() {
    local pids=()
    local failed=0
    for crate in "$@"; do
        publish_crate "$crate" &
        pids+=($!)
    done

    for pid in "${pids[@]}"; do
        if ! wait "$pid"; then
            failed=1
        fi
    done

    if [ $failed -eq 1 ]; then
        echo -e "${RED}One or more crates failed to publish in this group.${NC}"
        exit 1
    fi
}

# Group 1: Independent crates
echo -e "\n${GREEN}Publishing Group 1 (Independent)...${NC}"
run_parallel "ferrotunnel-common" "ferrotunnel-plugin" "ferrotunnel-protocol"

# Group 2: Dependent on Group 1
echo -e "\n${GREEN}Publishing Group 2...${NC}"
run_parallel "ferrotunnel-core" "ferrotunnel-observability"

# Group 3: Dependent on Group 2
echo -e "\n${GREEN}Publishing Group 3...${NC}"
publish_crate "ferrotunnel-http"

# Group 4: Final binaries and main crate
echo -e "\n${GREEN}Publishing Group 4 (Final binaries)...${NC}"
run_parallel "ferrotunnel" "ferrotunnel-cli"

echo -e "\n${GREEN}All crates published successfully!${NC}"
