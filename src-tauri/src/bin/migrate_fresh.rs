/// One-time migration reset tool
///
/// This tool creates a fresh migration history by:
/// 1. Backing up current database and migrations
/// 2. Extracting current schema from backup
/// 3. Creating a single initial migration with all current schema
/// 4. Creating fresh database with new migration
///
/// After running this, use refresh_data to repopulate data.

use anyhow::{Context, Result, bail};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::io::{self, Write};

fn main() -> Result<()> {
    println!("🔄 Migration Fresh - Start with Clean Migration History");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Check PROJECT_ROOT is set
    let project_root = env::var("PROJECT_ROOT")
        .context("❌ PROJECT_ROOT environment variable is not set.\n\nPlease set it before running:\n  export PROJECT_ROOT=/path/to/rust-stocks")?;

    let project_root_path = PathBuf::from(&project_root);
    if !project_root_path.exists() {
        bail!("❌ PROJECT_ROOT directory does not exist: {}", project_root);
    }

    // Load .env file from PROJECT_ROOT
    let env_file = project_root_path.join(".env");
    if !env_file.exists() {
        bail!("❌ .env file not found at: {}", env_file.display());
    }

    dotenvy::from_filename(&env_file)
        .context("Failed to load .env file")?;

    println!("✅ Loaded .env from: {}", env_file.display());

    // Get RUST_ROOT from .env
    let rust_root = env::var("RUST_ROOT")
        .context("❌ RUST_ROOT not set in .env file")?;

    let rust_root_path = PathBuf::from(&rust_root);
    if !rust_root_path.exists() {
        bail!("❌ RUST_ROOT directory does not exist: {}", rust_root);
    }

    // Check current directory matches RUST_ROOT
    let current_dir = env::current_dir()
        .context("Failed to get current directory")?;

    if current_dir != rust_root_path {
        bail!(
            "❌ Wrong directory!\n\nCurrent: {}\nExpected: {}\n\nPlease cd to RUST_ROOT before running.",
            current_dir.display(),
            rust_root_path.display()
        );
    }

    let db_path = current_dir.join("db/stocks.db");
    let migrations_dir = current_dir.join("db/migrations");

    // Verify database exists
    if !db_path.exists() {
        bail!("❌ Database not found at: {}", db_path.display());
    }

    // Verify migrations directory exists
    if !migrations_dir.exists() {
        bail!("❌ Migrations directory not found at: {}", migrations_dir.display());
    }

    println!("✅ Current directory: {}", current_dir.display());
    println!("✅ Database: {}", db_path.display());
    println!("✅ Migrations: {}", migrations_dir.display());
    println!();

    // Show what will happen
    println!("⚠️  This tool will:");
    println!("   1. Backup db/stocks.db → db/stocks.db.backup");
    println!("   2. Backup db/migrations/ → db/migrations_backup/");
    println!("   3. Extract current schema from backup database");
    println!("   4. Create fresh db/migrations/ with single initial migration");
    println!("   5. Create fresh database with new migration");
    println!();
    println!("⚠️  After this, you MUST run refresh_data to repopulate all data!");
    println!();

    // Confirmation prompt
    print!("Do you want to proceed? Type 'yes' to confirm: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "yes" {
        println!("❌ Aborted by user");
        return Ok(());
    }

    println!();
    println!("🚀 Starting migration fresh process...");
    println!();

    // Step 1: Backup database
    let db_backup_path = current_dir.join("db/stocks.db.backup");
    println!("📦 Backing up database...");
    fs::copy(&db_path, &db_backup_path)
        .context("Failed to backup database")?;
    println!("   ✅ Database backed up to: {}", db_backup_path.display());

    // Step 2: Backup migrations
    let migrations_backup_dir = current_dir.join("db/migrations_backup");
    if migrations_backup_dir.exists() {
        bail!(
            "❌ db/migrations_backup/ already exists!\n\n\
            Please remove or rename it before running migrate_fresh:\n  \
            rm -rf db/migrations_backup/\n\n\
            Location: {}",
            migrations_backup_dir.display()
        );
    }

    println!("📦 Backing up migrations...");
    fs::rename(&migrations_dir, &migrations_backup_dir)
        .context("Failed to backup migrations directory")?;
    println!("   ✅ Migrations backed up to: {}", migrations_backup_dir.display());

    // Step 3: Extract schema from backup database
    println!("📄 Extracting schema from backup database...");
    let schema_output = Command::new("sqlite3")
        .arg(&db_backup_path)
        .arg(".schema")
        .output()
        .context("Failed to execute sqlite3 .schema command. Is sqlite3 installed?")?;

    if !schema_output.status.success() {
        bail!("❌ Failed to extract schema from database");
    }

    let mut schema = String::from_utf8(schema_output.stdout)
        .context("Failed to parse schema output as UTF-8")?;

    // Remove _sqlx_migrations table from schema (sqlx creates this automatically)
    schema = remove_sqlx_migrations_table(&schema);

    println!("   ✅ Schema extracted ({} bytes)", schema.len());

    // Step 4: Create fresh migrations directory
    fs::create_dir_all(&migrations_dir)
        .context("Failed to create fresh migrations directory")?;
    println!("   ✅ Created fresh migrations directory");

    // Step 5: Create initial migration files using sqlx
    println!("📝 Creating initial migration files...");
    let migration_name = "initial_schema";

    let sqlx_output = Command::new("sqlx")
        .args(&["migrate", "add", "-r", migration_name, "--source", "db/migrations"])
        .current_dir(&current_dir)
        .output()
        .context("Failed to execute sqlx migrate add")?;

    if !sqlx_output.status.success() {
        let error = String::from_utf8_lossy(&sqlx_output.stderr);
        bail!("❌ Failed to create migration: {}", error);
    }

    // Find the created migration files
    let migration_files: Vec<_> = fs::read_dir(&migrations_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.contains("initial_schema") && name.ends_with(".up.sql")
        })
        .collect();

    if migration_files.is_empty() {
        bail!("❌ Failed to find created migration files");
    }

    let up_file = migration_files[0].path();
    // Correctly construct down.sql filename: replace .up.sql with .down.sql
    let up_filename = up_file.file_name().unwrap().to_string_lossy();
    let down_filename = up_filename.replace(".up.sql", ".down.sql");
    let down_file = migrations_dir.join(down_filename);

    println!("   ✅ Created migration files");
    println!("      - {}", up_file.file_name().unwrap().to_string_lossy());
    println!("      - {}", down_file.file_name().unwrap().to_string_lossy());

    // Step 6: Parse schema and create DROP statements for down migration
    println!("📝 Generating migration content...");

    let drop_statements = generate_drop_statements(&schema)?;

    // Write up migration (CREATE statements)
    let up_content = format!(
        "-- Initial schema migration\n\
         -- Created from existing database schema\n\
         -- This represents the complete schema at the time of migration reset\n\n\
         {}",
        schema
    );

    fs::write(&up_file, up_content)
        .context("Failed to write .up.sql file")?;
    println!("   ✅ Wrote CREATE statements to .up.sql ({} bytes)", schema.len());

    // Write down migration (DROP statements)
    let down_content = format!(
        "-- Revert: Drop all tables and views\n\
         -- This will completely remove the schema\n\n\
         {}",
        drop_statements
    );

    fs::write(&down_file, down_content)
        .context("Failed to write .down.sql file")?;
    println!("   ✅ Wrote DROP statements to .down.sql ({} bytes)", drop_statements.len());

    // Step 7: Delete old database
    println!("🗑️  Removing old database...");
    fs::remove_file(&db_path)
        .context("Failed to remove old database")?;
    println!("   ✅ Old database removed");

    // Step 8: Create empty database file
    println!("📝 Creating empty database...");
    let create_db_output = Command::new("sqlite3")
        .arg(&db_path)
        .arg("SELECT 1;")
        .output()
        .context("Failed to create empty database with sqlite3")?;

    if !create_db_output.status.success() {
        bail!("❌ Failed to create empty database");
    }
    println!("   ✅ Empty database created");

    // Step 9: Apply fresh migration
    println!("🚀 Applying fresh migration...");

    // Get DATABASE_URL from environment (loaded from .env)
    let database_url = env::var("DATABASE_URL")
        .context("❌ DATABASE_URL not set in .env file")?;

    let migrate_output = Command::new("sqlx")
        .args(&["migrate", "run", "--source", "db/migrations"])
        .current_dir(&current_dir)
        .env("DATABASE_URL", database_url)
        .output()
        .context("Failed to execute sqlx migrate run")?;

    if !migrate_output.status.success() {
        let error = String::from_utf8_lossy(&migrate_output.stderr);
        bail!("❌ Failed to apply migration: {}", error);
    }

    println!("   ✅ Migration applied successfully");

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Migration fresh complete!");
    println!();
    println!("📋 Summary:");
    println!("   - Old database: db/stocks.db.backup");
    println!("   - Old migrations: db/migrations_backup/");
    println!("   - New migrations: db/migrations/ (clean, reversible)");
    println!("   - New database: db/stocks.db (empty, schema only)");
    println!();
    println!("⚠️  NEXT STEPS:");
    println!("   1. Run: cargo run --bin refresh_data -- all");
    println!("   2. This will repopulate all S&P 500 data from SEC EDGAR");
    println!();

    Ok(())
}

/// Remove SQLite internal tables and indexes from schema
fn remove_sqlx_migrations_table(schema: &str) -> String {
    let mut result = String::new();
    let mut skip_statement = false;
    let mut paren_depth = 0;

    for line in schema.lines() {
        let trimmed = line.trim();

        // Skip SQLite internal/auto-generated objects:
        // - _sqlx_migrations (sqlx tracking table)
        // - sqlite_sequence (AUTOINCREMENT tracking)
        // - sqlite_autoindex_* (auto-generated indexes)
        let is_internal_table = trimmed.starts_with("CREATE TABLE") &&
            (trimmed.contains("_sqlx_migrations") ||
             trimmed.contains("sqlite_sequence"));

        let is_auto_index = (trimmed.starts_with("CREATE INDEX") || trimmed.starts_with("CREATE UNIQUE INDEX")) &&
            trimmed.contains("sqlite_autoindex_");

        if is_internal_table {
            skip_statement = true;
            paren_depth = 0;
        } else if is_auto_index {
            // Auto-indexes are single-line, skip the entire line
            continue;
        }

        // Track parentheses depth to handle multiline CREATE TABLE
        if skip_statement {
            for ch in line.chars() {
                match ch {
                    '(' => paren_depth += 1,
                    ')' => paren_depth -= 1,
                    _ => {}
                }
            }

            // End of CREATE TABLE when we close all parentheses
            if paren_depth == 0 && trimmed.ends_with(");") {
                skip_statement = false;
            }
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}

/// Generate DROP statements from schema
fn generate_drop_statements(schema: &str) -> Result<String> {
    let mut tables = Vec::new();
    let mut views = Vec::new();
    let mut indexes = Vec::new();

    // Parse schema to find all tables, views, and indexes
    for line in schema.lines() {
        let trimmed = line.trim();

        // Match CREATE TABLE statements
        if trimmed.starts_with("CREATE TABLE") {
            if let Some(table_name) = extract_object_name(trimmed, "CREATE TABLE") {
                tables.push(format!("DROP TABLE IF EXISTS {};", table_name));
            }
        }

        // Match CREATE VIEW statements
        if trimmed.starts_with("CREATE VIEW") {
            if let Some(view_name) = extract_object_name(trimmed, "CREATE VIEW") {
                views.push(format!("DROP VIEW IF EXISTS {};", view_name));
            }
        }

        // Match CREATE INDEX statements
        if trimmed.starts_with("CREATE INDEX") || trimmed.starts_with("CREATE UNIQUE INDEX") {
            let prefix = if trimmed.starts_with("CREATE UNIQUE INDEX") {
                "CREATE UNIQUE INDEX"
            } else {
                "CREATE INDEX"
            };
            if let Some(index_name) = extract_object_name(trimmed, prefix) {
                indexes.push(format!("DROP INDEX IF EXISTS {};", index_name));
            }
        }
    }

    // Drop in correct order: indexes first, then views, then tables
    // This ensures dependencies are respected
    let mut drops = Vec::new();
    drops.extend(indexes);
    drops.extend(views);
    drops.extend(tables);

    Ok(drops.join("\n"))
}

/// Extract object name from CREATE statement
fn extract_object_name(line: &str, prefix: &str) -> Option<String> {
    let after_prefix = line.strip_prefix(prefix)?.trim();

    // Handle "IF NOT EXISTS"
    let after_if_not_exists = if after_prefix.starts_with("IF NOT EXISTS") {
        after_prefix.strip_prefix("IF NOT EXISTS")?.trim()
    } else {
        after_prefix
    };

    // Get the name (first word before space or '(')
    let name = after_if_not_exists
        .split(|c: char| c.is_whitespace() || c == '(')
        .next()?
        .trim();

    // Remove quotes if present
    let name = name.trim_matches('"').trim_matches('`');

    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}
