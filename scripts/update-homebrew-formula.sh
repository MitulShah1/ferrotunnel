#!/bin/bash
# Script to update Homebrew formula SHA256 for a new release

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 1.0.1"
    exit 1
fi

VERSION=$1
FORMULA_FILE="Formula/ferrotunnel.rb"
TARBALL_URL="https://github.com/MitulShah1/ferrotunnel/archive/refs/tags/v${VERSION}.tar.gz"

echo "🔍 Fetching tarball and calculating SHA256..."
SHA256=$(curl -sL "$TARBALL_URL" | shasum -a 256 | awk '{print $1}')

if [ -z "$SHA256" ]; then
    echo "❌ Failed to calculate SHA256. Make sure the release v${VERSION} exists on GitHub."
    exit 1
fi

echo "✅ SHA256: $SHA256"

if [ ! -f "$FORMULA_FILE" ]; then
    echo "❌ Formula file not found: $FORMULA_FILE"
    exit 1
fi

echo "📝 Updating formula file..."

# Update the URL and SHA256 in the formula
sed -i.bak "s|url \".*\"|url \"$TARBALL_URL\"|" "$FORMULA_FILE"
sed -i.bak "s|sha256 \".*\"|sha256 \"$SHA256\"|" "$FORMULA_FILE"

rm "${FORMULA_FILE}.bak"

echo "✅ Formula updated!"
echo ""
echo "Next steps:"
echo "1. Review the changes: git diff $FORMULA_FILE"
echo "2. Test locally: brew install --build-from-source $FORMULA_FILE"
echo "3. Commit: git add $FORMULA_FILE && git commit -m 'Update formula for v${VERSION}'"
echo "4. Push to homebrew-ferrotunnel repo"
