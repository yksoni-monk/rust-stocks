#!/bin/bash

# Automatic Database Backup Script
# Run this before any migrations or major operations

DB_FILE="stocks.db"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_DIR="backups"
BACKUP_FILE="${BACKUP_DIR}/stocks_backup_${TIMESTAMP}.db"

# Create backup directory if it doesn't exist
mkdir -p "$BACKUP_DIR"

# Check if database exists
if [ ! -f "$DB_FILE" ]; then
    echo "❌ Database file not found: $DB_FILE"
    exit 1
fi

# Get database size
DB_SIZE=$(stat -f%z "$DB_FILE" 2>/dev/null || stat -c%s "$DB_FILE" 2>/dev/null)
DB_SIZE_MB=$((DB_SIZE / 1024 / 1024))

echo "📊 Database: $DB_FILE (${DB_SIZE_MB} MB)"
echo "💾 Creating backup: $BACKUP_FILE"

# Create backup using SQLite backup command
sqlite3 "$DB_FILE" ".backup '$BACKUP_FILE'"

if [ $? -eq 0 ]; then
    # Verify backup
    BACKUP_SIZE=$(stat -f%z "$BACKUP_FILE" 2>/dev/null || stat -c%s "$BACKUP_FILE" 2>/dev/null)
    BACKUP_SIZE_MB=$((BACKUP_SIZE / 1024 / 1024))
    
    if [ "$BACKUP_SIZE" -gt 0 ] && [ "$BACKUP_SIZE" -ge $((DB_SIZE / 2)) ]; then
        echo "✅ Backup created successfully: ${BACKUP_SIZE_MB} MB"
        echo "🔒 Backup location: $BACKUP_FILE"
    else
        echo "❌ Backup verification failed - suspicious file size"
        rm -f "$BACKUP_FILE"
        exit 1
    fi
else
    echo "❌ Backup failed"
    exit 1
fi

# Keep only last 5 backups
echo "🧹 Cleaning old backups (keeping last 5)..."
ls -t "${BACKUP_DIR}"/stocks_backup_*.db 2>/dev/null | tail -n +6 | xargs -r rm

echo "✅ Backup complete: $BACKUP_FILE"