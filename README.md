# Rust Stocks Analysis System

🚨 **PRODUCTION DATABASE IS `src-tauri/db/stocks.db` (2.5GB)** 🚨

A high-performance desktop application for stock analysis using Tauri (Rust backend + SolidJS frontend) with EDGAR API integration. Features comprehensive financial data, advanced screening algorithms, and enterprise-grade database safeguards.

## 📊 Advanced Screening Algorithms (PRODUCTION READY)
✅ **Piotroski F-Score**: 9-criteria financial strength scoring for S&P 500 stocks
✅ **O'Shaughnessy Value**: 6-metric value composite screening with dynamic ranking
✅ **EDGAR API Integration**: Real-time financial data from SEC filings
✅ **Data Quality Assessment**: Completeness scoring and freshness monitoring

## 🚀 Quick Start - Desktop Application

```bash
# Clone and run the Tauri desktop application
git clone <repo>
cd rust-stocks
npm run tauri dev
```

**That's it!** The modern desktop application with advanced screening algorithms will open automatically.

## 🎯 Project Overview

This system provides comprehensive stock market data analysis capabilities, featuring:
- ✅ **EDGAR API Integration**: Real-time financial data from SEC filings
- ✅ **Advanced Screening Algorithms**: Piotroski F-Score and O'Shaughnessy Value methods
- ✅ **SolidJS Frontend**: Modern reactive UI with TypeScript
- ✅ **S&P 500 Focus**: Comprehensive coverage of 503 S&P 500 stocks
- ✅ **Enterprise Database Safeguards**: Production-grade backup and migration system
- ✅ **Data Quality Assessment**: Completeness scoring and freshness monitoring
- ✅ **Dynamic Screening**: Configurable criteria with real-time results

## 🎯 Product Features

### 📊 Advanced Screening & Analysis
- **Piotroski F-Score**: 9-criteria financial strength scoring (profitability, leverage, efficiency)
- **O'Shaughnessy Value**: 6-metric value composite screening (P/B, P/S, P/CF, P/E, EV/EBITDA, Shareholder Yield)
- **Dynamic Criteria**: Configurable screening thresholds with real-time updates
- **Data Quality Scoring**: Completeness assessment for reliable analysis
- **S&P 500 Focus**: Comprehensive coverage of 503 S&P 500 stocks

### 📈 Interactive Stock Analysis
- **Screening Results Display**: Real-time results with detailed metrics and reasoning
- **Data Quality Indicators**: Completeness scores and freshness status
- **Individual Stock Analysis**: Detailed financial metrics and historical data
- **Export Capabilities**: Export screening results and analysis data
- **Responsive Design**: Modern UI with smooth animations and professional styling

### 🔍 Data Management & System Status
- **Data Refresh System**: Real-time data updates from EDGAR API
- **System Status Monitoring**: Data freshness and completeness tracking
- **S&P 500 Symbol Management**: Automatic symbol updates and sector classification
- **Database Health Monitoring**: Production database safeguards and backup system
- **Error Handling**: Comprehensive error reporting and recovery mechanisms

### 💼 Professional Data Integration
- **SEC EDGAR API**: Real-time financial data from official SEC filings
- **Multi-Year Financial Data**: 5+ years of historical financial statements
- **Data Quality Assessment**: Completeness scoring and validation
- **Production Database**: 2.5GB database with enterprise-grade safeguards
- **Automated Backup System**: Timestamped backups with integrity verification

### 🖥️ Modern Desktop Experience
- **Tauri Desktop App**: Native performance with web technologies (Rust + SolidJS)
- **SolidJS Frontend**: Modern reactive UI with TypeScript
- **Responsive Design**: Smooth animations and professional user interface
- **Cross-Platform**: Windows, macOS, and Linux support
- **Real-Time Updates**: Live screening results with dynamic criteria updates

### 📤 Data Export & Integration
- **Screening Results Export**: Export Piotroski and O'Shaughnessy screening results
- **Financial Data Export**: Export underlying financial statement data
- **Data Quality Reports**: Export completeness and freshness reports
- **Future API Integration**: Charles Schwab API integration planned for real-time quotes

## 🛠️ Setup & Installation

### Prerequisites
- **Node.js and npm** (for Tauri desktop application)
- **Rust** (latest stable version)
- **SQLite** (bundled with the application)
- **Python 3** (for Schwab API authentication)
- **Schwab API credentials** (from [Schwab Developer Portal](https://developer.schwab.com/))

### Setup
1. Clone the repository
2. Install dependencies:
   ```bash
   npm install
   ```
3. Configure environment variables:
   ```bash
   cp .env.example .env
   # Edit .env with your actual Schwab API credentials
   ```
4. Install Python dependencies for Schwab API:
   ```bash
   pip install schwab-py
   ```
5. Authenticate with Schwab API:
   ```bash
   python3 refresh_token.py --auth
   ```
6. Build the project:
   ```bash
   cargo build --release
   ```
7. Run the desktop application:
   ```bash
   npm run tauri:dev
   ```

The application will automatically fetch S&P 500 symbols and connect to the EDGAR API for financial data.

### First-Time Setup (Database Initialization)
If you're running the project for the first time, you'll need to initialize the database:

1. **Check database status**:
   ```bash
   cd src-tauri
   cargo run --bin db_admin -- status
   ```

2. **Run database migrations** (if needed):
   ```bash
   cd src-tauri
   cargo run --bin db_admin -- migrate --confirm
   ```
   
   **About Migrations**: The project uses `sqlx` migrations to manage database schema changes. All 36 migrations will be applied in chronological order to build the complete database schema from scratch. Each migration is tracked in the `_sqlx_migrations` table with a checksum to ensure integrity. Once applied, migrations should never be modified.

3. **Refresh data** (to populate with S&P 500 data):
   ```bash
   cd src-tauri
   cargo run --bin refresh_data -- financials
   cargo run --bin refresh_data -- ratios
   ```

**Note**: The first data refresh may take 15-30 minutes as it downloads financial data from the SEC EDGAR API for all S&P 500 stocks.

### Schwab API Setup
To use the market data refresh functionality, you'll need to set up Schwab API credentials:

1. **Get Schwab API credentials**:
   - Visit [Schwab Developer Portal](https://developer.schwab.com/)
   - Create an account and register your application
   - Copy your API Key and App Secret

2. **Configure credentials**:
   - Edit `.env` file with your actual Schwab API credentials
   - Update `PROJECT_ROOT` with your actual project path

3. **Authenticate** (one-time setup):
   ```bash
   python3 refresh_token.py --auth
   ```
   - Follow the browser authentication flow
   - Paste the redirect URL when prompted

4. **Refresh tokens** (as needed):
   ```bash
   python3 refresh_token.py --refresh
   ```

**Note**: Schwab API tokens expire and need periodic refresh. The system will automatically refresh tokens when needed.

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

### Database Migrations

The project uses **`sqlx` migrations** for all database schema changes. This provides:

- **Version Control**: Every schema change is tracked in timestamped migration files
- **Checksum Validation**: Each migration has a SHA-256 checksum to ensure integrity
- **Sequential Application**: Migrations run in chronological order
- **Idempotency**: Re-running migrations has no effect if already applied
- **Rollback Protection**: Applied migrations should never be modified

#### Migration Workflow

**For New Databases** (first-time setup):
```bash
cd src-tauri
sqlx migrate run --database-url "sqlite:db/stocks.db" --source db/migrations
```
All 36 migrations will execute in order to create the complete schema.

**For Existing Databases**:
```bash
cd src-tauri
sqlx migrate info --database-url "sqlite:db/stocks.db" --source db/migrations
```
This shows which migrations are installed. Only new migrations will be applied.

**Creating New Migrations**:
```bash
cd src-tauri
sqlx migrate add --source db/migrations descriptive_name
```
This creates a new migration file with a timestamp prefix.

**Important Rules**:
- ❌ **NEVER modify** an applied migration file (breaks checksum)
- ✅ **Always create** new migrations for schema changes
- ✅ **Test migrations** on a backup database first
- ✅ **Backup first** before running migrations on production data

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

## 📊 EDGAR API Data System

### Data Processing Overview
The system uses real-time EDGAR API integration with automated data processing:

1. **S&P 500 Symbol Management**: Automatic symbol updates from GitHub repository
2. **EDGAR Financial Data Extraction**: Hybrid approach using Submissions + Company Facts APIs
3. **10-K/A Amendment Handling**: Automatic detection and replacement with corrected data
4. **Financial Ratio Calculations**: Piotroski F-Score and O'Shaughnessy Value calculations
5. **Data Quality Assessment**: Completeness scoring and validation
6. **Performance Optimization**: Database indexing for fast screening queries

### Data Sources & Quality
- **SEC EDGAR API**: Official financial data from SEC filings
- **Coverage**: S&P 500 stocks with comprehensive financial data
- **Frequency**: Real-time API calls with local caching
- **Quality**: Official SEC data used by financial institutions
- **Format**: JSON API responses with structured financial data

### Data Refresh Commands

#### Unified Data Refresh System
The system provides intelligent data refresh with automatic freshness detection:

```bash
# Check data status (default - no refresh)
cargo run --bin refresh_data

# Show detailed status with verbose output
cargo run --bin refresh_data -- --status --verbose

# Preview what would be refreshed (dry-run mode)
cargo run --bin refresh_data -- --preview market
cargo run --bin refresh_data -- --preview financials
```

#### Refresh Modes

**Market Data Refresh** (Schwab API - Daily Prices):
```bash
# Refresh daily stock prices from Schwab API
cargo run --bin refresh_data market

# Expected: 2-5 minutes for 497 S&P 500 stocks
# Updates: daily_prices table with latest market data
```

**Financial Data Refresh** (SEC EDGAR API - Financial Statements):
```bash
# Refresh financial statements from SEC EDGAR API
cargo run --bin refresh_data financials

# Expected: 15-30 minutes for full S&P 500 (497 stocks with CIKs)
# Updates: balance_sheets, income_statements, cash_flow_statements
# Features:
#   - Hybrid API approach (Submissions API for metadata + Company Facts API for data)
#   - 10-K/A amendment support with automatic upsert (replaces original with corrected data)
#   - Atomic transactions (all-or-nothing storage)
#   - No orphaned sec_filing records
#   - Automatic duplicate handling and deduplication
#   - 10 concurrent workers with rate limiting (10 req/sec)
```

**Single Stock Testing** (Fast debugging):
```bash
# Test with single stock (faster for debugging)
cargo run --bin refresh_data financials --only-ticker AAPL
cargo run --bin refresh_data market --only-ticker WMT

# Expected: <1 second per stock
# Skips freshness checks for faster execution
```

#### Command Options

**Status & Preview**:
- `--status` or `-s`: Show current data freshness status
- `--preview` or `-p`: Show what would be refreshed (dry-run)
- `--verbose` or `-v`: Show detailed progress information

**Filtering**:
- `--only-ticker SYMBOL`: Process only specific stock ticker
- `--only-cik CIK`: Process only specific SEC CIK number

**Examples**:
```bash
# Check if data needs refresh
cargo run --bin refresh_data --status

# See what would be refreshed for market data
cargo run --bin refresh_data --preview market

# Refresh financials with detailed output
cargo run --bin refresh_data financials --verbose

# Test EDGAR API with single stock
cargo run --bin refresh_data financials --only-ticker MSFT
```

#### Data Freshness Detection

The system automatically detects stale data:

**Market Data**:
- ✅ Current: Updated within 7 days
- ⚠️ Stale: Older than 7 days
- ❌ Missing: No data in database

**Financial Data**:
- ✅ Current: All SEC filing dates present
- ⚠️ Stale: Missing filing dates from SEC
- ❌ Missing: No financial data

**Screening Readiness**:
- Shows which screening algorithms are ready to run
- Blocks if required data is stale or missing
- Provides commands to refresh specific data types

### Expected Performance & Data

**Full S&P 500 Refresh**:
- **Market Data**: 2-5 minutes (497 stocks, Schwab API)
- **Financial Data**: 15-30 minutes (497 stocks, EDGAR API)
- **Concurrency**: 10 parallel workers with rate limiting
- **Rate Limits**: 10 requests/second (SEC EDGAR requirement)

**Single Stock Testing**:
- **Market Data**: <1 second per stock
- **Financial Data**: <1 second per stock
- **Ideal for**: Debugging, testing, development

**Database Impact**:
- **Financial Data**: ~503 stocks with 5+ years historical data
- **Database Size**: 2.5GB with complete financial statements
- **Stock Coverage**: S&P 500 companies (497 with CIK identifiers)
- **Data Integrity**: ACID-compliant atomic storage (no orphaned records)
- **Amendment Handling**: 10-K/A filings automatically replace original 10-K data

## 🎨 Application Features

### Modern Desktop Interface
- **SolidJS Frontend**: Modern reactive UI with TypeScript
- **Screening Interface**: Piotroski F-Score and O'Shaughnessy Value methods
- **Real-Time Results**: Dynamic screening with configurable criteria
- **Data Quality Indicators**: Completeness scores and freshness status
- **Responsive Design**: Smooth animations and professional styling
- **Cross-Platform**: Desktop application for Windows, macOS, Linux

### Stock Analysis Capabilities
- **Advanced Screening**: 9-criteria Piotroski F-Score and 6-metric O'Shaughnessy Value
- **Data Quality Assessment**: Completeness scoring and validation
- **Individual Stock Analysis**: Detailed financial metrics and historical data
- **Export Capabilities**: Export screening results and analysis data
- **Real-Time Updates**: Live data refresh from EDGAR API

### Technical Features
- **High Performance**: Handles S&P 500 screening with real-time results
- **Professional UI**: Modern design with smooth animations
- **Production Ready**: Enterprise-grade database safeguards
- **TypeScript Integration**: Full type safety across frontend and backend

## 🛠️ Available Commands

### Application Commands
```bash
# Run desktop application  
npm run tauri:dev

# Run database admin tool (from src-tauri directory)
cd src-tauri
cargo run --bin db_admin -- status
cargo run --bin db_admin -- backup  
cargo run --bin db_admin -- migrate --confirm
cargo run --bin db_admin -- verify

# Data refresh commands (from src-tauri directory)
cd src-tauri
cargo run --bin refresh_data -- --help
cargo run --bin refresh_data -- financials
cargo run --bin refresh_data -- ratios

# Schwab API token management
python3 refresh_token.py --auth    # Initial authentication
python3 refresh_token.py --refresh # Refresh expired tokens
```

### Database Commands (from src-tauri directory)
```bash
cd src-tauri

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
- **Frontend**: SolidJS with TypeScript, reactive UI
- **Backend**: Rust with Tauri framework
- **Database**: SQLite with professional-grade safeguards
- **Data Source**: SEC EDGAR API integration (real-time)
- **Future Integration**: Charles Schwab API (for real-time quotes and options)

### Key Components
- **EDGAR API Client**: Hybrid API approach (Submissions + Company Facts)
  - Submissions API: 10-K filing metadata with form types and accession numbers
  - Company Facts API: Financial statement data extraction
  - 10-K/A amendment detection and automatic upsert
- **Database Manager**: Enterprise-grade backup and migration system
- **Screening Engine**: Piotroski F-Score and O'Shaughnessy Value algorithms
- **Data Quality System**: Completeness scoring and validation
- **S&P 500 Integration**: Symbol management and sector classification

## 📚 Documentation

All documentation is centralized in this README for simplicity:

- **Setup & Installation**: See sections above for complete setup guide
- **Database Administration**: Enterprise-grade backup and migration tools
- **EDGAR API Integration**: Real-time financial data system
- **Application Features**: Modern screening interface and analysis capabilities
- **Architecture**: Technology stack and system components

For detailed technical architecture, see: `context/ARCHITECTURE.md`

---

*Last Updated: 2025-01-01*  
*Version: 4.0 - EDGAR API Integration with Advanced Screening Algorithms*  
*SolidJS Frontend with Piotroski F-Score and O'Shaughnessy Value Methods*
