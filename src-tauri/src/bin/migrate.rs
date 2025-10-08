/// Migration helper tool for rust-stocks project
///
/// This tool ensures proper migration workflow using sqlx CLI.
/// Always use this tool instead of manually running sqlx commands.
///
/// Prerequisites:
/// - RUST_ROOT environment variable must be set
/// - Must be run from RUST_ROOT directory
/// - .env file must exist with DATABASE_URL and MIGRATION_PATH

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::env;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "migrate")]
#[command(about = "Migration helper for rust-stocks database", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new reversible migration (creates .up.sql and .down.sql files)
    Create {
        /// Name of the migration (e.g., fix_piotroski_use_annual_data)
        name: String,
    },
    /// Apply all pending migrations
    Run,
    /// Revert the last applied migration
    Revert,
    /// Show migration status
    Status,
    /// Show migration info from sqlx
    Info,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check PROJECT_ROOT is set
    let project_root = env::var("PROJECT_ROOT")
        .context("❌ PROJECT_ROOT environment variable is not set.\n\nPlease set it before running migrations:\n  export PROJECT_ROOT=/path/to/rust-stocks")?;

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
            "❌ Wrong directory!\n\nCurrent: {}\nExpected: {}\n\nPlease cd to RUST_ROOT before running migrations.",
            current_dir.display(),
            rust_root_path.display()
        );
    }

    println!("✅ Current directory: {}", current_dir.display());

    // Get configuration from .env
    let database_url = env::var("DATABASE_URL")
        .context("❌ DATABASE_URL not set in .env file")?;

    let migration_path = env::var("MIGRATION_PATH")
        .context("❌ MIGRATION_PATH not set in .env file")?;

    let migration_dir = current_dir.join(&migration_path);
    if !migration_dir.exists() {
        bail!("❌ Migration directory does not exist: {}", migration_dir.display());
    }

    println!("✅ DATABASE_URL: {}", database_url);
    println!("✅ MIGRATION_PATH: {}", migration_path);
    println!();

    match cli.command {
        Commands::Create { name } => {
            create_migration(&name, &migration_path)?;
        }
        Commands::Run => {
            run_migrations(&migration_path)?;
        }
        Commands::Revert => {
            revert_migration(&migration_path)?;
        }
        Commands::Status => {
            show_status(&migration_path)?;
        }
        Commands::Info => {
            show_info(&migration_path)?;
        }
    }

    Ok(())
}

fn create_migration(name: &str, migration_path: &str) -> Result<()> {
    println!("📝 Creating reversible migration: {}", name);
    println!();

    let status = Command::new("sqlx")
        .args(&["migrate", "add", "-r", name, "--source", migration_path])
        .status()
        .context("Failed to execute sqlx migrate add. Is sqlx-cli installed?")?;

    if !status.success() {
        bail!("❌ Failed to create migration");
    }

    println!();
    println!("✅ Migration files created in {}/", migration_path);
    println!();
    println!("⚠️  IMPORTANT NEXT STEPS:");
    println!("   1. Edit both .up.sql and .down.sql files");
    println!("   2. Test your SQL in a dev environment first");
    println!("   3. Run 'cargo run --bin migrate run' to apply migration");
    println!("   4. DO NOT manually apply migrations with sqlite3");
    println!("   5. DO NOT edit migration files after running 'migrate run'");
    println!();

    Ok(())
}

fn run_migrations(migration_path: &str) -> Result<()> {
    println!("🚀 Applying pending migrations...");
    println!();

    let status = Command::new("sqlx")
        .args(&["migrate", "run", "--source", migration_path])
        .status()
        .context("Failed to execute sqlx migrate run")?;

    if !status.success() {
        bail!("❌ Failed to apply migrations");
    }

    println!();
    println!("✅ Migrations applied successfully");
    Ok(())
}

fn revert_migration(migration_path: &str) -> Result<()> {
    println!("⏪ Reverting last migration...");
    println!();

    let status = Command::new("sqlx")
        .args(&["migrate", "revert", "--source", migration_path])
        .status()
        .context("Failed to execute sqlx migrate revert")?;

    if !status.success() {
        bail!("❌ Failed to revert migration");
    }

    println!();
    println!("✅ Migration reverted successfully");
    Ok(())
}

fn show_status(migration_path: &str) -> Result<()> {
    println!("📊 Migration status:");
    println!();

    // Show sqlx info
    let status = Command::new("sqlx")
        .args(&["migrate", "info", "--source", migration_path])
        .status()
        .context("Failed to execute sqlx migrate info")?;

    if !status.success() {
        bail!("❌ Failed to get migration status");
    }

    Ok(())
}

fn show_info(migration_path: &str) -> Result<()> {
    println!("ℹ️  Migration info:");
    println!();

    let status = Command::new("sqlx")
        .args(&["migrate", "info", "--source", migration_path])
        .status()
        .context("Failed to execute sqlx migrate info")?;

    if !status.success() {
        bail!("❌ Failed to get migration info");
    }

    Ok(())
}
