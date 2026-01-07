#!/bin/bash
# Backup script for Txt-code project
# Creates a timestamped commit with all changes

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_DIR"

# Get timestamp
TIMESTAMP=$(date +"%Y-%m-%d_%H-%M-%S")
COMMIT_MSG="Backup: $(date +"%Y-%m-%d %H:%M:%S")"

# Add all changes
git add -A

# Check if there are changes to commit
if git diff --staged --quiet; then
    echo "No changes to commit"
    exit 0
fi

# Create commit
git commit -m "$COMMIT_MSG" || {
    echo "Failed to create commit"
    exit 1
}

echo "Backup created: $COMMIT_MSG"
echo "Commit hash: $(git rev-parse --short HEAD)"

# Optional: Create archive
if [ "$1" = "--archive" ]; then
    ARCHIVE_NAME="backup_${TIMESTAMP}.tar.gz"
    git archive --format=tar.gz --output="${ARCHIVE_NAME}" HEAD
    echo "Archive created: ${ARCHIVE_NAME}"
fi

