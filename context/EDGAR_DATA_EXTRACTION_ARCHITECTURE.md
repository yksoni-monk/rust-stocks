# EDGAR Data Extraction - Implementation Architecture

## 📋 Executive Summary

Comprehensive architecture for extracting financial data from the local EDGAR dataset (18,915 companies) and populating our database with income statements and balance sheets. This will provide superior data coverage (3x more companies) and complete the foundation for our screening algorithms.

## 🎯 Objectives

### Primary Goals
1. **Data Coverage**: Extract financial data for 18,915+ companies from EDGAR files
2. **Schema Population**: Populate `income_statements` and `balance_sheets` tables  
3. **Data Quality**: Ensure accurate mapping from EDGAR GAAP fields to database schema
4. **Performance**: Efficient batch processing of large JSON files
5. **Integration**: Seamless integration with existing screening algorithms

### Success Metrics
- ✅ 18,915+ companies with complete financial statements
- ✅ 3x increase in data coverage vs current 5,892 companies  
- ✅ 100% data accuracy with validation against known values (Apple Q3 2024)
- ✅ All screening algorithms functional with new dataset
- ✅ Processing completion within 2-4 hours

## 🏗️ System Architecture

### High-Level Design
```
┌─────────────────────────────────────────────────────────────────────────┐
│                    EDGAR Data Extraction System                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   EDGAR File    │    │   CIK-to-Symbol  │    │   GAAP Field     │   │
│  │   Discovery     │───▶│   Mapper         │───▶│   Transformer    │   │
│  │                 │    │                  │    │                  │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
│           │                        │                        │           │
│           ▼                        ▼                        ▼           │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   JSON Parser   │    │   Data           │    │   Database       │   │
│  │   & Validator   │◀──▶│   Validator      │───▶│   Batch Writer   │   │
│  │                 │    │                  │    │                  │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
│           │                        │                        │           │
│           ▼                        ▼                        ▼           │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   Period        │    │   Quality        │    │   Progress       │   │
│  │   Processor     │    │   Assurance      │    │   Tracker        │   │
│  │                 │    │                  │    │                  │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. EDGAR File Discovery
```rust
struct EdgarFileDiscovery {
    base_path: PathBuf,
    file_patterns: Vec<String>,
    batch_size: usize,
}
```

**Responsibilities:**
- Scan `/edgar_data/companyfacts/` directory for CIK*.json files
- Batch file processing for memory efficiency
- Track progress across 18,915 files
- Handle corrupted or malformed files gracefully

#### 2. CIK-to-Symbol Mapper
```rust
struct CikSymbolMapper {
    cik_to_symbol: HashMap<String, String>,
    symbol_to_stock_id: HashMap<String, i64>,
    unmapped_ciks: HashSet<String>,
}
```

**Responsibilities:**
- Map EDGAR CIK numbers to stock symbols
- Handle multiple symbols per CIK (Class A/B shares)
- Track unmapped CIKs for analysis
- Validate symbol existence in stocks table

#### 3. GAAP Field Transformer
```rust
struct GaapFieldTransformer {
    income_statement_mapping: HashMap<String, String>,
    balance_sheet_mapping: HashMap<String, String>,
    field_priorities: HashMap<String, Vec<String>>,
}
```

**Responsibilities:**
- Map EDGAR GAAP field names to database columns
- Handle field variations and alternatives
- Apply transformation logic for calculations
- Validate field data types and ranges

#### 4. Period Processor
```rust
struct PeriodProcessor {
    target_periods: Vec<String>,  // ["Q1", "Q2", "Q3", "Q4", "FY"]
    date_validators: Vec<DateValidator>,
    ttm_calculator: TtmCalculator,
}
```

**Responsibilities:**
- Extract quarterly and annual financial data
- Calculate TTM (Trailing Twelve Months) values
- Handle fiscal year variations
- Normalize reporting periods

#### 5. Database Batch Writer
```rust
struct DatabaseBatchWriter {
    db_pool: SqlitePool,
    batch_size: usize,
    transaction_timeout: Duration,
    conflict_strategy: ConflictStrategy,
}
```

**Responsibilities:**
- Efficient batch insertion of financial statements
- Handle duplicate period conflicts (UPSERT)
- Maintain referential integrity
- Provide rollback capability for errors

## 📊 Data Mapping Strategy

### EDGAR JSON Structure Analysis
```json
{
  "cik": 320193,
  "entityName": "Apple Inc.",
  "facts": {
    "us-gaap": {
      "RevenueFromContractWithCustomerExcludingAssessedTax": {
        "units": {
          "USD": [
            {
              "end": "2024-06-29",
              "val": 85777000000,
              "form": "10-Q",
              "fy": 2024,
              "fp": "Q3"
            }
          ]
        }
      }
    }
  }
}
```

### Income Statement Field Mapping
| Database Column | Primary GAAP Field | Alternative GAAP Fields | Transformation |
|---|---|---|---|
| `revenue` | `RevenueFromContractWithCustomerExcludingAssessedTax` | `SalesRevenueNet`, `Revenues` | Direct mapping |
| `net_income` | `NetIncomeLoss` | `NetIncomeLossAvailableToCommonStockholdersBasic` | Direct mapping |
| `operating_income` | `IncomeLossFromContinuingOperations` | `OperatingIncomeLoss` | Direct mapping |
| `shares_basic` | `WeightedAverageNumberOfSharesOutstandingBasic` | `CommonStockSharesOutstanding` | Direct mapping |
| `shares_diluted` | `WeightedAverageNumberOfDilutedSharesOutstanding` | `WeightedAverageNumberOfSharesOutstandingBasic` | Fallback to basic |

### Balance Sheet Field Mapping
| Database Column | Primary GAAP Field | Alternative GAAP Fields | Transformation |
|---|---|---|---|
| `total_assets` | `Assets` | `AssetsTotal` | Direct mapping |
| `total_debt` | `LongTermDebt` + `DebtCurrent` | `DebtAndCapitalLeaseObligations` | Sum components |
| `total_equity` | `StockholdersEquity` | `ShareholdersEquity` | Direct mapping |
| `cash_and_equivalents` | `CashAndCashEquivalentsAtCarryingValue` | `CashCashEquivalentsAndShortTermInvestments` | Direct mapping |
| `shares_outstanding` | `CommonStockSharesOutstanding` | `CommonStockSharesIssued` | Direct mapping |

### Field Priority System
```rust
// Priority-based field selection for robust data extraction
let revenue_fields = vec![
    "RevenueFromContractWithCustomerExcludingAssessedTax",
    "SalesRevenueNet", 
    "Revenues",
    "RevenueFromContractWithCustomerIncludingAssessedTax"
];

// Select first available field with valid data
for field in revenue_fields {
    if let Some(value) = extract_field_value(&gaap_facts, field, &period) {
        income_statement.revenue = Some(value);
        break;
    }
}
```

## 🔄 Data Processing Pipeline

### Phase 1: Discovery and Inventory
1. **File Scanning**: Scan `/edgar_data/companyfacts/` for all CIK*.json files
2. **Size Analysis**: Calculate total data volume and estimated processing time
3. **CIK Extraction**: Build initial CIK inventory from filenames
4. **Validation**: Verify JSON file integrity and structure

### Phase 2: CIK-to-Symbol Mapping
```rust
// Multiple strategies for CIK-to-symbol mapping
async fn build_cik_symbol_mapping(db_pool: &SqlitePool) -> Result<HashMap<String, String>> {
    let mut cik_map = HashMap::new();
    
    // Strategy 1: Direct SEC symbol lookup (if available)
    // Strategy 2: Company name fuzzy matching
    // Strategy 3: External CIK mapping services
    // Strategy 4: Manual mapping for top companies
    
    Ok(cik_map)
}
```

**Mapping Strategies:**
1. **Direct Lookup**: Use existing SEC symbol files if available
2. **Fuzzy Matching**: Match EDGAR entity names to database company names
3. **Manual Mapping**: Hard-code mappings for Fortune 500 companies
4. **Progressive Enhancement**: Start with high-confidence mappings, expand iteratively

### Phase 3: Data Extraction
```rust
async fn extract_financial_data(cik_file: &Path, cik_map: &CikSymbolMapper) -> Result<ExtractedData> {
    // 1. Parse JSON file
    let edgar_data: EdgarCompanyFacts = serde_json::from_str(&file_content)?;
    
    // 2. Map CIK to symbol and stock_id
    let stock_id = cik_map.get_stock_id(&edgar_data.cik)?;
    
    // 3. Extract income statement data for each period
    let income_statements = extract_income_statements(&edgar_data.facts, stock_id)?;
    
    // 4. Extract balance sheet data for each period
    let balance_sheets = extract_balance_sheets(&edgar_data.facts, stock_id)?;
    
    // 5. Validate data quality
    validate_financial_data(&income_statements, &balance_sheets)?;
    
    Ok(ExtractedData {
        income_statements,
        balance_sheets,
    })
}
```

### Phase 4: Database Integration
```rust
async fn batch_insert_financial_data(
    db_pool: &SqlitePool,
    extracted_data: Vec<ExtractedData>,
) -> Result<()> {
    let mut tx = db_pool.begin().await?;
    
    // Batch insert income statements
    for income_stmt in extracted_data.iter().flat_map(|d| &d.income_statements) {
        sqlx::query!(
            "INSERT OR REPLACE INTO income_statements (...) VALUES (...)",
            // ... bind values
        )
        .execute(&mut *tx)
        .await?;
    }
    
    // Batch insert balance sheets
    for balance_sheet in extracted_data.iter().flat_map(|d| &d.balance_sheets) {
        sqlx::query!(
            "INSERT OR REPLACE INTO balance_sheets (...) VALUES (...)",
            // ... bind values
        )
        .execute(&mut *tx)
        .await?;
    }
    
    tx.commit().await?;
    Ok(())
}
```

## 📈 Performance Optimization

### Memory Management
- **Streaming Processing**: Process one EDGAR file at a time
- **Batch Size Optimization**: 100-500 companies per database transaction
- **Memory Monitoring**: Track memory usage and trigger GC if needed
- **File Handle Management**: Properly close file handles to prevent leaks

### Processing Speed
- **Parallel Processing**: Process multiple files concurrently (limited by memory)
- **Database Connection Pooling**: Maintain 5-10 database connections
- **Prepared Statements**: Cache SQL statements for reuse
- **Index Optimization**: Ensure proper database indexes for performance

### Storage Efficiency
- **Incremental Updates**: Only update changed financial data
- **Data Compression**: Use SQLite compression for large datasets
- **Index Strategy**: Balance query performance vs storage size
- **Cleanup Strategy**: Remove obsolete or invalid records

## 📊 Data Quality Assurance

### Validation Rules
```rust
fn validate_income_statement(stmt: &IncomeStatement) -> ValidationResult {
    let mut issues = Vec::new();
    
    // Revenue validation
    if let Some(revenue) = stmt.revenue {
        if revenue < 0.0 {
            issues.push("Negative revenue detected".to_string());
        }
        if revenue > 1_000_000_000_000.0 {  // $1T threshold
            issues.push("Suspiciously high revenue".to_string());
        }
    }
    
    // Net income validation
    if let (Some(revenue), Some(net_income)) = (stmt.revenue, stmt.net_income) {
        let margin = net_income / revenue;
        if margin < -1.0 || margin > 1.0 {
            issues.push("Unusual profit margin detected".to_string());
        }
    }
    
    // Share count validation
    if let Some(shares) = stmt.shares_basic {
        if shares <= 0 {
            issues.push("Invalid share count".to_string());
        }
    }
    
    if issues.is_empty() {
        ValidationResult::Valid
    } else {
        ValidationResult::Warning(issues)
    }
}
```

### Quality Metrics
1. **Completeness**: Percentage of required fields populated
2. **Consistency**: Cross-validation between related fields
3. **Accuracy**: Spot-check against known values (Apple Q3 2024)
4. **Coverage**: Number of companies with complete financial data

### Data Validation Strategy
```rust
// Multi-level validation approach
struct DataValidator {
    // Level 1: Structural validation
    json_validator: JsonValidator,
    
    // Level 2: Business logic validation  
    financial_validator: FinancialValidator,
    
    // Level 3: Cross-reference validation
    benchmark_validator: BenchmarkValidator,
}

impl DataValidator {
    async fn validate_extracted_data(&self, data: &ExtractedData) -> ValidationResult {
        // Combine all validation results
        let structural = self.json_validator.validate(&data.raw_json)?;
        let financial = self.financial_validator.validate(&data.statements)?;
        let benchmark = self.benchmark_validator.validate(&data.company_metrics)?;
        
        ValidationResult::combine(vec![structural, financial, benchmark])
    }
}
```

## 🛠️ Implementation Phases

### Phase 1: Infrastructure Setup (Day 1)
1. **Project Structure**: Create `import-edgar-data` binary
2. **Dependencies**: Add required JSON parsing and file I/O dependencies
3. **Database Schema**: Ensure income_statements and balance_sheets tables are ready
4. **File Discovery**: Implement EDGAR file scanning and inventory
5. **Progress Tracking**: Basic progress display and logging

**Deliverable**: Tool that can scan EDGAR files and show processing progress

### Phase 2: CIK Mapping Implementation (Day 1-2)
1. **Symbol Mapping**: Build CIK-to-symbol mapping strategies
2. **Fuzzy Matching**: Implement company name matching algorithms
3. **Manual Mappings**: Create hard-coded mappings for major companies
4. **Validation**: Verify mapping accuracy with known companies
5. **Error Handling**: Handle unmapped CIKs gracefully

**Deliverable**: Robust CIK-to-symbol mapping with >95% coverage for major companies

### Phase 3: Data Extraction Engine (Day 2-3)
1. **JSON Parsing**: Implement EDGAR JSON structure parsing
2. **Field Mapping**: Build GAAP field to database column mapping
3. **Period Processing**: Extract quarterly and annual financial data
4. **Data Transformation**: Apply necessary calculations and conversions
5. **Quality Validation**: Implement comprehensive data validation

**Deliverable**: Complete data extraction pipeline with quality checks

### Phase 4: Database Integration (Day 3)
1. **Batch Insertion**: Efficient database writing with transactions
2. **Conflict Resolution**: Handle duplicate period data (UPSERT)
3. **Performance Optimization**: Connection pooling and prepared statements
4. **Error Recovery**: Rollback capability for failed batches
5. **Progress Persistence**: Save progress for resume capability

**Deliverable**: Production-ready database integration with error recovery

### Phase 5: Testing & Validation (Day 3-4)
1. **Unit Tests**: Test core components individually
2. **Integration Tests**: End-to-end pipeline testing
3. **Data Validation**: Verify extracted data against known benchmarks
4. **Performance Testing**: Measure processing speed and memory usage
5. **Quality Assessment**: Comprehensive data quality analysis

**Deliverable**: Thoroughly tested system with quality metrics

### Phase 6: Production Deployment (Day 4)
1. **Configuration**: Production environment setup
2. **Monitoring**: Progress tracking and logging
3. **Backup Strategy**: Database backup before bulk import
4. **Execution**: Full EDGAR dataset processing
5. **Validation**: Post-import data quality verification

**Deliverable**: Complete EDGAR financial data integrated into database

## 🔍 Error Handling & Recovery

### Error Categories

#### 1. File Processing Errors
- **Corrupted JSON**: Skip file, log error, continue processing
- **Missing Files**: Log missing CIK numbers, continue with available files
- **Permission Issues**: Check file permissions, retry with appropriate access

#### 2. Data Mapping Errors
- **Unknown CIK**: Track unmapped CIKs for later manual mapping
- **Invalid Symbols**: Validate symbols against stocks table
- **Multiple Mappings**: Handle CIKs mapping to multiple symbols

#### 3. Data Validation Errors
- **Missing Fields**: Use fallback fields or mark as null
- **Invalid Values**: Apply data cleaning rules or skip record
- **Inconsistent Data**: Flag for manual review, use best available data

#### 4. Database Errors
- **Connection Issues**: Retry with exponential backoff
- **Constraint Violations**: Handle conflicts with UPSERT strategy
- **Transaction Failures**: Rollback and retry batch

### Recovery Strategies

#### Progress Persistence
```json
{
  "session_id": "edgar-extraction-uuid",
  "start_time": "2024-09-18T10:00:00Z",
  "total_files": 18915,
  "processed_files": 5420,
  "successful_extractions": 5201,
  "failed_extractions": 219,
  "current_batch": "batch_054",
  "cik_mapping_stats": {
    "mapped_ciks": 5201,
    "unmapped_ciks": 219,
    "mapping_confidence": 95.9
  },
  "data_quality_metrics": {
    "complete_income_statements": 4987,
    "complete_balance_sheets": 4923,
    "validation_warnings": 156,
    "validation_errors": 38
  }
}
```

#### Resume Capability
- **File-level Resume**: Skip already processed CIK files
- **Batch-level Resume**: Resume from last committed database batch
- **Incremental Processing**: Only process files newer than last run
- **Selective Retry**: Retry only failed extractions from previous run

## 📋 Quality Control Framework

### Benchmark Validation
```rust
// Validate against known Apple Q3 2024 data
async fn validate_apple_q3_2024(db_pool: &SqlitePool) -> Result<ValidationReport> {
    let apple_data = sqlx::query!(
        "SELECT revenue, net_income FROM income_statements 
         WHERE stock_id = (SELECT id FROM stocks WHERE symbol = 'AAPL')
         AND period = 'Q3' AND year = 2024"
    )
    .fetch_one(db_pool)
    .await?;
    
    let expected_revenue = 85_777_000_000; // $85.777B
    let expected_net_income = 21_448_000_000; // $21.448B
    
    let revenue_match = (apple_data.revenue - expected_revenue).abs() < 1_000_000; // $1M tolerance
    let income_match = (apple_data.net_income - expected_net_income).abs() < 1_000_000;
    
    ValidationReport {
        company: "Apple Inc.".to_string(),
        period: "Q3 2024".to_string(),
        revenue_accuracy: if revenue_match { "EXACT" } else { "MISMATCH" },
        income_accuracy: if income_match { "EXACT" } else { "MISMATCH" },
        confidence: if revenue_match && income_match { "HIGH" } else { "LOW" },
    }
}
```

### Coverage Analysis
```rust
// Comprehensive coverage analysis
struct CoverageAnalysis {
    total_edgar_companies: usize,
    mapped_companies: usize,
    complete_financials: usize,
    incomplete_financials: usize,
    coverage_by_sector: HashMap<String, f64>,
    data_completeness_score: f64,
}

impl CoverageAnalysis {
    async fn generate(db_pool: &SqlitePool) -> Result<Self> {
        // Analyze data coverage across multiple dimensions
        // Compare against previous SimFin data coverage
        // Identify coverage gaps and improvement opportunities
    }
}
```

## 🎯 Success Criteria

### Primary Objectives
- ✅ **Data Volume**: 18,915+ companies with financial data (3x improvement)
- ✅ **Data Quality**: >99% accuracy validated against benchmarks
- ✅ **Coverage**: >95% of S&P 500 companies with complete financial statements
- ✅ **Performance**: Complete processing within 4 hours
- ✅ **Integration**: All screening algorithms functional with new data

### Quality Benchmarks
- ✅ **Apple Validation**: Perfect match for Q3 2024 revenue and net income
- ✅ **Field Completeness**: >90% of core fields populated across companies
- ✅ **Historical Depth**: Multi-year financial data for trend analysis
- ✅ **Data Consistency**: Cross-validation between income and balance sheet data

### Performance Targets
- ✅ **Processing Speed**: >1,000 companies per hour
- ✅ **Memory Usage**: <4GB peak memory consumption
- ✅ **Database Growth**: Efficient storage with proper indexing
- ✅ **Error Rate**: <1% unrecoverable processing errors

## 🔧 Configuration & Usage

### Environment Variables
```bash
# Required
DATABASE_URL=sqlite:./db/stocks.db
EDGAR_DATA_PATH=/Users/yksoni/code/misc/rust-stocks/edgar_data/companyfacts

# Optional
BATCH_SIZE=500
MAX_CONCURRENT_FILES=10
PROGRESS_FILE=./edgar_extraction_progress.json
LOG_LEVEL=info
VALIDATION_LEVEL=strict
```

### CLI Interface
```bash
# Basic usage
cargo run --bin import-edgar-data

# Custom data path
cargo run --bin import-edgar-data --edgar-path /path/to/edgar/data

# Resume interrupted processing
cargo run --bin import-edgar-data --resume

# Test mode (single CIK file)
cargo run --bin import-edgar-data --test-cik 320193

# Validation only (no extraction)
cargo run --bin import-edgar-data --validate-only

# Specific company batch
cargo run --bin import-edgar-data --cik-range 1000-2000
```

### Monitoring Dashboard
```
🏗️ EDGAR Financial Data Extraction
===================================
Status: RUNNING | Session: edgar-abc123
Started: 2024-09-18 10:00:00 | Runtime: 2h 15m

File Processing:
├─ Total Files: 18,915
├─ Processed: 12,340 (65.2%) ████████████████████████████
├─ Successful: 11,987 (97.1%)
├─ Failed: 353 (2.9%)
├─ Remaining: 6,575 (34.8%)

CIK Mapping:
├─ Mapped CIKs: 11,234 (93.7%)
├─ Unmapped CIKs: 753 (6.3%)
├─ Symbol Coverage: 94.2%
├─ Confidence Level: HIGH

Data Extraction:
├─ Income Statements: 45,678 records
├─ Balance Sheets: 43,234 records  
├─ Complete Companies: 11,087 (92.6%)
├─ Data Quality Score: 96.8%

Performance:
├─ Processing Speed: 1,234 companies/hour
├─ Memory Usage: 2.8GB / 4GB (70%)
├─ Database Size: +892MB
├─ ETA: 5h 20m
```

## 🏁 Expected Outcomes

### Data Foundation Enhancement
- **18,915+ companies** with comprehensive financial statements
- **3x data coverage** improvement over current SimFin dataset
- **Superior data quality** from official SEC filings vs third-party aggregation
- **Cost efficiency** through one-time extraction vs ongoing API costs

### Screening Algorithm Enhancement
- **GARP Screening**: Enhanced with 3x more company coverage
- **Graham Value Screening**: More opportunities in expanded universe
- **P/S and P/E Analysis**: Comprehensive ratio calculations across broader market
- **Sector Analysis**: Complete industry coverage for comparative analysis

### Strategic Capabilities
- **Market Coverage**: Support for mid-cap and small-cap stock analysis
- **Historical Analysis**: Multi-year financial trends for all companies
- **Regulatory Compliance**: SEC-validated financial data for institutional use
- **Competitive Analysis**: Complete financial profiles for industry comparisons

### Performance Benefits
- **Query Performance**: Local data access vs API dependency
- **Data Reliability**: Consistent data quality vs mixed-source aggregation
- **Operational Efficiency**: No rate limits or API costs
- **Scalability**: Foundation for advanced analytics and machine learning

This architecture provides a robust, scalable foundation for extracting the complete EDGAR financial dataset while maintaining the highest standards of data quality and processing efficiency. The implementation will transform our screening capabilities through superior data coverage and reliability.