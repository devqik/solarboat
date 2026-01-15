#!/bin/bash

# Script to update Homebrew formula after a new release
# Usage: ./update-homebrew.sh <version> <macos_x86_sha256> <macos_arm_sha256> <linux_x86_sha256> <linux_arm_sha256>

set -e

VERSION=$1
MACOS_X86_SHA256=$2
MACOS_ARM_SHA256=$3
LINUX_X86_SHA256=$4
LINUX_ARM_SHA256=$5

if [[ -z "$VERSION" || -z "$MACOS_X86_SHA256" || -z "$MACOS_ARM_SHA256" || -z "$LINUX_X86_SHA256" || -z "$LINUX_ARM_SHA256" ]]; then
    echo "Usage: $0 <version> <macos_x86_sha256> <macos_arm_sha256> <linux_x86_sha256> <linux_arm_sha256>"
    echo ""
    echo "Example:"
    echo "  $0 0.8.0 abc123... def456... ghi789... jkl012..."
    exit 1
fi

echo "Updating Homebrew formula for solarboat v$VERSION"

# Get the repository dispatch URL (targeting the Homebrew tap repository)
TAP_REPO_OWNER="devqik"
TAP_REPO_NAME="homebrew-solarboat"
DISPATCH_URL="https://api.github.com/repos/$TAP_REPO_OWNER/$TAP_REPO_NAME/dispatches"

# Create the payload
PAYLOAD=$(cat <<EOF
{
  "event_type": "update-formula",
  "client_payload": {
    "version": "$VERSION",
    "macos_x86_sha256": "$MACOS_X86_SHA256",
    "macos_arm_sha256": "$MACOS_ARM_SHA256",
    "linux_x86_sha256": "$LINUX_X86_SHA256",
    "linux_arm_sha256": "$LINUX_ARM_SHA256"
  }
}
EOF
)

# Send the repository dispatch event
if [[ -n "$GITHUB_TOKEN" ]]; then
    echo "Sending repository dispatch event..."
    curl -X POST \
        -H "Accept: application/vnd.github.v3+json" \
        -H "Authorization: token $GITHUB_TOKEN" \
        -H "Content-Type: application/json" \
        "$DISPATCH_URL" \
        -d "$PAYLOAD"

    echo ""
    echo "✅ Repository dispatch event sent successfully!"
    echo "The Homebrew formula will be updated automatically."
    echo "Check: https://github.com/$TAP_REPO_OWNER/$TAP_REPO_NAME/actions"
else
    echo "⚠️  GITHUB_TOKEN environment variable not set."
    echo "You can manually trigger the update by running:"
    echo ""
    echo "curl -X POST \\"
    echo "  -H \"Accept: application/vnd.github.v3+json\" \\"
    echo "  -H \"Authorization: token \$GITHUB_TOKEN\" \\"
    echo "  -H \"Content-Type: application/json\" \\"
    echo "  \"$DISPATCH_URL\" \\"
    echo "  -d '$PAYLOAD'"
fi
