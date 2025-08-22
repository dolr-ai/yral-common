#!/bin/bash
#
# Setup script to configure git hooks for the project
#

echo "Setting up git hooks for the project..."

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "Error: This is not a git repository."
    exit 1
fi

# Configure git to use the .githooks directory
git config core.hooksPath .githooks

# Create .githooks directory if it doesn't exist
mkdir -p .githooks

# Write the pre-commit hook
cat > .githooks/pre-commit <<'EOF'
#!/usr/bin/env bash

set -e

echo "Running cargo check --all-features..."
cargo check --all-features
if [ $? -ne 0 ]; then
  echo "cargo check failed. Commit aborted."
  exit 1
fi

echo "Running cargo clippy --no-deps --all-features --release -- -Dwarnings..."
cargo clippy --no-deps --all-features --release -- -Dwarnings
if [ $? -ne 0 ]; then
  echo "cargo clippy failed. Commit aborted."
  exit 1
fi
EOF

# Make sure the hooks are executable
chmod +x .githooks/*

echo "Git hooks configured successfully!"
echo "Now build checks will run automatically on every commit."
echo "To bypass checks (not recommended), use: git commit --no-verify"
echo "To run checks manually: .githooks/pre-commit"
