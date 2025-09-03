#!/bin/bash
# Clean up test tmp directory

echo "🧹 Cleaning up test tmp directory..."

if [ -d "tests/tmp" ]; then
    rm -rf tests/tmp/*
    echo "✅ Cleaned up tests/tmp directory"
else
    echo "ℹ️  tests/tmp directory doesn't exist, creating it"
    mkdir -p tests/tmp
fi

echo "✅ Test cleanup complete!"
