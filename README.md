# Rust Stocks Analysis System

🚨 **CLAUDE: PRODUCTION DATABASE IS `./stocks.db` (2.5GB) IN THIS ROOT DIRECTORY** 🚨

A high-performance desktop application for stock analysis using Tauri (Rust backend + React frontend) with offline-first architecture. Features comprehensive stock data from SimFin, expandable panels UI, and enterprise-grade database safeguards.

## 📊 P/S & EV/S Ratio System (COMPLETED)
✅ **3,294 P/S and EV/S ratios** calculated for stocks where P/E ratios are invalid due to negative earnings
✅ **54,160 TTM financial statements** imported from SimFin data
✅ **96.4% data completeness** - production ready

## 🚀 Quick Start - Desktop Application

```bash
# Clone and run the Tauri desktop application
git clone <repo>
cd rust-stocks
npm run tauri:dev
```

**That's it!** The modern desktop application with expandable panels UI will open automatically.

## 🎯 Project Overview

This system provides comprehensive stock market data analysis capabilities, featuring:
- ✅ **Offline-First Architecture**: 5,876+ stocks with comprehensive historical data (2019-2024)
- ✅ **SimFin Data Integration**: Professional-grade financial data with automated import
- ✅ **Expandable Panels UI**: Modern single-page interface with contextual expansion
- ✅ **User-Driven Analysis**: Dynamic metric selection (P/E, EPS, Price, Volume, etc.)
- ✅ **Enterprise Database Safeguards**: Production-grade backup and migration system
- ✅ **High-Performance Processing**: 6.2M price records + 52k+ financial records
- ✅ **S&P 500 Support**: Integrated filtering and offline symbol management

## 🛠️ Setup & Data Import

### Prerequisites
- **Node.js and npm** (for Tauri desktop application)
- **Rust** (latest stable version)
- **SimFin CSV data files** (for comprehensive stock data)
- **SQLite** (bundled with the application)

### Setup
1. Clone the repository
2. Download SimFin data files (us-shareprices-daily.csv, us-income-quarterly.csv)
3. Import data using the SimFin importer:
   ```bash
   cargo run --bin import-simfin -- \
     --prices ~/simfin_data/us-shareprices-daily.csv \
     --income ~/simfin_data/us-income-quarterly.csv \
     --db stocks.db
   ```
4. Build the project:
   ```bash
   cargo build --release
   ```
5. Run the desktop application:
   ```bash
   npm run tauri:dev
   ```

## 🔒 Database Administration & Safety

### Enterprise-Grade Database Protection
This system includes comprehensive database safeguards to prevent accidental data loss:

#### Database Admin Tool
```bash
# Check database health and statistics
cargo run --bin db_admin -- status

# Create manual backup
cargo run --bin db_admin -- backup

# Run migrations with safety checks
cargo run --bin db_admin -- migrate --confirm

# Verify database integrity
cargo run --bin db_admin -- verify
```

#### Automatic Backup System
- **Production Detection**: Automatically detects databases >50MB or >1000 stocks
- **Pre-Migration Backups**: Creates backups before any schema changes
- **Backup Verification**: Validates backup file size and integrity
- **Timestamped Backups**: Automatic cleanup keeps last 5 backups

#### Shell Backup Script
```bash
# Manual backup script
./backup_database.sh

# Creates: backups/stocks_backup_YYYYMMDD_HHMMSS.db
```

### Safety Features
- 🔒 **Production Database Protection**: Requires explicit confirmation for large databases
- 📦 **Automatic Backups**: Created before any potentially destructive operations  
- ✅ **Data Integrity Verification**: Post-migration data validation
- 🚨 **Rollback Support**: Easy restoration from timestamped backups
- 📊 **Health Monitoring**: Real-time database statistics and alerts

## 📊 SimFin Data Import System

### Import Process Overview
The system uses a 6-phase automated import process:

1. **Phase 1**: Extract unique stocks from daily prices CSV
2. **Phase 2**: Import daily price records (OHLCV + shares outstanding)
3. **Phase 3**: Import quarterly financial statements 
4. **Phase 4**: Calculate EPS (Net Income ÷ Diluted Shares Outstanding)
5. **Phase 5**: Calculate P/E ratios (Close Price ÷ Latest Available EPS)
6. **Phase 6**: Create performance indexes for fast queries

### Import Commands

#### Recommended Usage (From Project Root)
```bash
# Full import from project root directory
cargo run --bin import-simfin -- \
  --prices ~/simfin_data/us-shareprices-daily.csv \
  --income ~/simfin_data/us-income-quarterly.csv \
  --db stocks.db

# With custom batch size for performance tuning
cargo run --bin import-simfin -- \
  --prices ~/simfin_data/us-shareprices-daily.csv \
  --income ~/simfin_data/us-income-quarterly.csv \
  --db stocks.db \
  --batch-size 5000

# Show help and all available options
cargo run --bin import-simfin -- --help
```

#### Alternative: Direct Method (From src-tauri Directory)
```bash
cd src-tauri
cargo run --bin import_simfin -- \
  --prices ~/simfin_data/us-shareprices-daily.csv \
  --income ~/simfin_data/us-income-quarterly.csv \
  --db ../stocks.db
```

#### Command Parameter Details
- `cargo run --bin import-simfin` - Runs the SimFin import tool
- `--` - **IMPORTANT**: Separates Cargo's arguments from application arguments
- `--prices [file]` - Path to daily prices CSV (semicolon-delimited)
- `--income [file]` - Path to quarterly income CSV (semicolon-delimited)  
- `--db [file]` - Path to SQLite database (optional, defaults to `./stocks.db`)
- `--batch-size [size]` - Records per batch (optional, defaults to 10,000)

### Expected Performance & Data
- **Processing Time**: 15-30 minutes for full dataset
- **Daily Prices**: ~6.2M records, 5,876+ stocks, 2019-2024
- **Quarterly Income**: ~52k financial records with comprehensive metrics
- **Final Database Size**: 2-3 GB
- **Stock Coverage**: 5,876+ companies with complete historical data

### Data Sources & Quality
- **SimFin**: High-quality financial data for 5,000+ companies
- **Coverage**: US stocks with comprehensive historical data
- **Frequency**: Daily prices, quarterly financials
- **Quality**: Professional-grade data used by financial institutions
- **Format**: Semicolon-delimited CSV files (SimFin standard)

### Troubleshooting

#### Command Issues
```bash
# If command not found, ensure you're in the right directory
cd /Users/yksoni/code/misc/rust-stocks

# Build the binary first if needed
cargo build --bin import_simfin
```

#### CSV Format Issues
- Ensure CSVs use semicolon (`;`) delimiters (SimFin format)
- Check that file paths are correct and files exist
- Verify CSV headers match expected SimFin format

#### Database Issues
- Ensure database schema is compatible (use db_admin tool)
- Check disk space (need ~3GB free for import + processing)
- Make sure no other processes are using the database file
- Use backup system before importing: `cargo run --bin db_admin -- backup`

#### Alternative Binary Usage
```bash
# Build and run directly (if needed)
cargo build --bin import_simfin
./target/debug/import_simfin \
  --prices ~/simfin_data/us-shareprices-daily.csv \
  --income ~/simfin_data/us-income-quarterly.csv \
  --db stocks.db
```

## 🎨 Application Features

### Modern Desktop Interface
- **Expandable Panels UI**: Single-page interface with contextual expansion
- **User-Driven Analysis**: Dynamic metric selection (P/E, EPS, Price, Volume, etc.)
- **S&P 500 Filtering**: Toggle between all stocks and S&P 500 subset
- **Real-Time Search**: Search stocks by symbol or company name  
- **Paginated Loading**: Efficient loading of large datasets
- **Visual Indicators**: 📊 for stocks with data, 📋 for no data
- **Multiple Panel Support**: Compare stocks side-by-side

### Stock Analysis Capabilities
- **Price History Visualization**: Interactive charts with historical data
- **Fundamental Analysis**: P/E ratios, EPS, market cap, financial metrics
- **Comparative Analysis**: Multiple stocks expanded simultaneously
- **Data Export**: CSV and JSON export for any selected view
- **Offline Operation**: Full functionality without internet connectivity

### Technical Features
- **High Performance**: Handles 6.2M+ price records smoothly
- **Professional UI**: Smooth animations and responsive design
- **Production Ready**: Enterprise-grade database safeguards
- **Cross-Platform**: Desktop application for Windows, macOS, Linux

## 🛠️ Available Commands

### Application Commands
```bash
# Run desktop application  
npm run tauri:dev

# Run database admin tool
cargo run --bin db_admin -- status
cargo run --bin db_admin -- backup  
cargo run --bin db_admin -- migrate --confirm
cargo run --bin db_admin -- verify

# Import SimFin data
cargo run --bin import-simfin -- --help
```

### Database Commands  
```bash
# Check database status
cargo run --bin db_admin -- status

# Create backup manually
cargo run --bin db_admin -- backup

# Run safe migrations
cargo run --bin db_admin -- migrate --confirm

# Verify database integrity
cargo run --bin db_admin -- verify
```

## 🏗️ Architecture

### Technology Stack
- **Frontend**: React with JSX, expandable panels UI
- **Backend**: Rust with Tauri framework
- **Database**: SQLite with professional-grade safeguards
- **Data Source**: SimFin CSV import system (offline-first)
- **Future Integration**: Charles Schwab API (for real-time quotes and options)

### Key Components
- **SimFin Importer**: 6-phase automated data import
- **Database Manager**: Enterprise-grade backup and migration system
- **Expandable Panels UI**: Modern single-page interface
- **Analysis Engine**: Dynamic metric calculations and visualizations
- **S&P 500 Integration**: Offline symbol management and filtering

## 📚 Documentation

All documentation is centralized in this README for simplicity:

- **Setup & Installation**: See sections above for complete setup guide
- **Database Administration**: Enterprise-grade backup and migration tools
- **SimFin Data Import**: Comprehensive import system with troubleshooting
- **Application Features**: Modern expandable panels UI and analysis capabilities
- **Architecture**: Technology stack and system components

For detailed technical architecture, see: `context/ARCHITECTURE.md`

---

*Last Updated: 2025-09-09*  
*Version: 3.0 - SimFin Offline Architecture with Enterprise Database Safeguards*  
*Integrated SimFin Import Documentation - Single README for clarity*
