#!/bin/bash
# Yank all published versions of FerroTunnel crates from crates.io
#
# Usage: ./scripts/yank-all.sh [--dry-run]
#
# Prerequisites:
#   - cargo login (must be authenticated)
#   - curl and jq installed
#
# Options:
#   --dry-run    Show what would be yanked without actually yanking

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

DRY_RUN=false
if [[ "$1" == "--dry-run" ]]; then
    DRY_RUN=true
    echo -e "${YELLOW}DRY RUN MODE - No changes will be made${NC}"
    echo ""
fi

# All FerroTunnel crate names (including old ones that may have been published)
CRATES=(
    "ferrotunnel"
    "ferrotunnel-common"
    "ferrotunnel-protocol"
    "ferrotunnel-core"
    "ferrotunnel-http"
    "ferrotunnel-plugin"
    "ferrotunnel-observability"
    "ferrotunnel-client"
    "ferrotunnel-server"
    "ferrotunnel-cli"
)

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║          FerroTunnel Crate Yanker                          ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check for required tools
if ! command -v jq &> /dev/null; then
    echo -e "${RED}Error: jq is required but not installed.${NC}"
    echo "Install with: sudo apt-get install jq"
    exit 1
fi

# Function to get all versions of a crate from crates.io
get_versions() {
    local crate_name=$1
    local response

    response=$(curl -s "https://crates.io/api/v1/crates/${crate_name}/versions" \
        -H "User-Agent: FerroTunnel-Yank-Script (https://github.com/MitulShah1/ferrotunnel)")

    # Check if crate exists
    if echo "$response" | jq -e '.errors' > /dev/null 2>&1; then
        echo ""
        return
    fi

    # Get all non-yanked versions
    echo "$response" | jq -r '.versions[] | select(.yanked == false) | .num' 2>/dev/null || echo ""
}

# Function to yank a specific version
yank_version() {
    local crate_name=$1
    local version=$2

    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "  ${YELLOW}[DRY RUN]${NC} Would yank ${crate_name}@${version}"
    else
        echo -e "  Yanking ${crate_name}@${version}..."
        if cargo yank "${crate_name}" --version "${version}" 2>/dev/null; then
            echo -e "  ${GREEN}✓${NC} Yanked ${crate_name}@${version}"
        else
            echo -e "  ${RED}✗${NC} Failed to yank ${crate_name}@${version}"
        fi
    fi
}

# Track statistics
TOTAL_YANKED=0
TOTAL_CRATES=0

# Process each crate
for crate in "${CRATES[@]}"; do
    echo -e "${YELLOW}Checking ${crate}...${NC}"

    versions=$(get_versions "$crate")

    if [[ -z "$versions" ]]; then
        echo -e "  ${BLUE}No published versions found${NC}"
        echo ""
        continue
    fi

    ((TOTAL_CRATES++))

    # Yank each version
    while IFS= read -r version; do
        if [[ -n "$version" ]]; then
            yank_version "$crate" "$version"
            ((TOTAL_YANKED++))
        fi
    done <<< "$versions"

    echo ""
done

# Summary
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
if [[ "$DRY_RUN" == "true" ]]; then
    echo -e "${YELLOW}DRY RUN COMPLETE${NC}"
    echo -e "Would yank ${TOTAL_YANKED} versions across ${TOTAL_CRATES} crates"
    echo ""
    echo -e "Run without --dry-run to actually yank:"
    echo -e "  ${BLUE}./scripts/yank-all.sh${NC}"
else
    echo -e "${GREEN}YANK COMPLETE${NC}"
    echo -e "Yanked ${TOTAL_YANKED} versions across ${TOTAL_CRATES} crates"
fi
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
