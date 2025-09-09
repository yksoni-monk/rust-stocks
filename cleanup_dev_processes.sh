#!/bin/bash

# Cleanup script for development processes
# This script kills any orphaned development server processes

echo "🧹 Cleaning up development processes..."

# Get the directory where this script is located (project root)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_SRC_DIR="$SCRIPT_DIR/src"

# Kill npm run dev processes
pkill -f "npm run dev" 2>/dev/null && echo "✅ Killed npm run dev processes"

# Kill vite processes
pkill -f "vite" 2>/dev/null && echo "✅ Killed vite processes"

# Kill node processes in our project directory (relative to script location)
pkill -f "$PROJECT_SRC_DIR" 2>/dev/null && echo "✅ Killed project node processes"

# Kill esbuild processes
pkill -f "esbuild" 2>/dev/null && echo "✅ Killed esbuild processes"

echo "🎉 Cleanup complete!"
