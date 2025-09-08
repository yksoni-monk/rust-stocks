# SimFin Data Import Guide

This document explains how to import fresh SimFin data into the stock analysis system.

## Quick Start (From Root Directory - Recommended)

```bash
# From /Users/yksoni/code/misc/rust-stocks/ (project root)
cargo run --bin import-simfin -- \
  --prices ~/simfin_data/us-shareprices-daily.csv \
  --income ~/simfin_data/us-income-quarterly.csv \
  --db stocks.db
```

## Alternative: Direct Method

```bash
# From src-tauri subdirectory
cd src-tauri
cargo run --bin import_simfin -- \
  --prices ~/simfin_data/us-shareprices-daily.csv \
  --income ~/simfin_data/us-income-quarterly.csv \
  --db ../stocks.db
```

## Available Commands

### From Root Directory (Convenient)

```bash
# Run main application
cargo run

# Import SimFin data
cargo run --bin import-simfin -- [OPTIONS]

# Show import help
cargo run --bin import-simfin
```

### From src-tauri Directory (Direct)

```bash
# Run main application  
cargo run --bin rust-stocks-tauri

# Import SimFin data
cargo run --bin import_simfin -- [OPTIONS]
```

## Command Breakdown

- `cargo run --bin import-simfin` - Runs the SimFin import tool (note: hyphen vs underscore)
- `--` - **IMPORTANT**: Separates Cargo's arguments from your application's arguments
- `--prices [file]` - Path to daily prices CSV (semicolon-delimited)
- `--income [file]` - Path to quarterly income CSV (semicolon-delimited)
- `--db [file]` - Path to SQLite database (optional, defaults to `./stocks.db`)

## Import Process

The tool performs these steps automatically:

1. **Phase 1**: Extract unique stocks from daily prices CSV
2. **Phase 2**: Import daily price records (OHLCV + shares outstanding)
3. **Phase 3**: Import quarterly financial statements 
4. **Phase 4**: Calculate EPS (Net Income ÷ Diluted Shares Outstanding)
5. **Phase 5**: Calculate P/E ratios (Close Price ÷ Latest Available EPS)
6. **Phase 6**: Create performance indexes for fast queries

## Expected Data

- **Daily Prices**: ~6.2M records, 5,876+ stocks, 2019-2024
- **Quarterly Income**: ~52k financial records with comprehensive metrics
- **Processing Time**: 15-30 minutes for full dataset
- **Final Database Size**: ~2-3 GB

## Troubleshooting

### Command Not Found
```bash
# Make sure you're in the right directory
cd /Users/yksoni/code/misc/rust-stocks/src-tauri

# Build the binary first if needed
cargo build --bin import_simfin
```

### CSV Format Issues
- Ensure CSVs use semicolon (`;`) delimiters (SimFin format)
- Check that file paths are correct and files exist
- Verify CSV headers match expected SimFin format

### Database Issues
- Ensure database schema has been migrated with `database_migration_simfin.sql`
- Check disk space (need ~3GB free for import + processing)
- Make sure no other processes are using the database file

## Alternative Usage

If you prefer using the pre-built binary:

```bash
# First build the binary
cargo build --bin import_simfin

# Then run it directly
./target/debug/import_simfin \
  --prices ~/simfin_data/us-shareprices-daily.csv \
  --income ~/simfin_data/us-income-quarterly.csv \
  --db ../stocks.db
```

## Help Command

```bash
cargo run --bin import_simfin -- --help
```

## Data Sources

- **SimFin**: High-quality financial data for 5,000+ companies
- **Coverage**: US stocks with comprehensive historical data
- **Frequency**: Daily prices, quarterly financials
- **Quality**: Professional-grade data used by financial institutions

---

**Last Updated**: 2025-09-08
**Tool Location**: `src-tauri/src/bin/import_simfin.rs`
**Schema Migration**: `database_migration_simfin.sql`