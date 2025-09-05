use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitProgress {
    pub current_step: String,
    pub companies_processed: usize,
    pub total_companies: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockData {
    pub symbol: String,
    pub company_name: String,
    pub sector: Option<String>,
}

async fn get_database_connection() -> Result<SqlitePool, String> {
    let database_url = "sqlite:../stocks.db";
    SqlitePool::connect(database_url).await
        .map_err(|e| format!("Database connection failed: {}", e))
}

#[tauri::command]
pub async fn initialize_sp500_stocks() -> Result<String, String> {
    let pool = get_database_connection().await?;
    
    // Step 1: Fetch S&P 500 list from GitHub
    let url = "https://raw.githubusercontent.com/datasets/s-and-p-500-companies/main/data/constituents.csv";
    
    let response = reqwest::get(url).await
        .map_err(|e| format!("Failed to fetch S&P 500 data: {}", e))?;
    
    let csv_text = response.text().await
        .map_err(|e| format!("Failed to read CSV data: {}", e))?;
    
    // Step 2: Parse CSV data
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let mut companies = Vec::new();
    
    for result in reader.records() {
        let record = result.map_err(|e| format!("CSV parsing error: {}", e))?;
        if record.len() >= 2 {
            let symbol = record[0].trim().to_string();
            let name = record[1].trim().to_string(); // "Security" column
            let sector = record.get(2).unwrap_or("").trim().to_string(); // "GICS Sector" column
            
            companies.push(StockData {
                symbol,
                company_name: name,
                sector: if sector.is_empty() { None } else { Some(sector) },
            });
        }
    }
    
    if companies.is_empty() {
        return Err("No companies found in S&P 500 data".to_string());
    }
    
    // Step 3: Clear existing stocks and insert new ones
    sqlx::query("DELETE FROM stocks")
        .execute(&pool).await
        .map_err(|e| format!("Failed to clear existing stocks: {}", e))?;
    
    let mut inserted = 0;
    for company in &companies {
        match sqlx::query(
            "INSERT INTO stocks (symbol, company_name, sector) VALUES (?1, ?2, ?3)"
        )
        .bind(&company.symbol)
        .bind(&company.company_name)
        .bind(&company.sector)
        .execute(&pool).await
        {
            Ok(_) => inserted += 1,
            Err(e) => eprintln!("Failed to insert {}: {}", company.symbol, e),
        }
    }
    
    // Step 4: Update metadata
    let current_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    sqlx::query("INSERT OR REPLACE INTO metadata (key, value) VALUES ('sp500_last_updated', ?1)")
        .bind(&current_date)
        .execute(&pool).await
        .map_err(|e| format!("Failed to update metadata: {}", e))?;
    
    let message = format!(
        "Successfully initialized {} S&P 500 companies out of {} found in dataset. Last updated: {}",
        inserted, companies.len(), current_date
    );
    
    Ok(message)
}

#[tauri::command]
pub async fn get_initialization_status() -> Result<InitProgress, String> {
    let pool = get_database_connection().await?;
    
    // Check if stocks are initialized
    let stock_count = match sqlx::query("SELECT COUNT(*) as count FROM stocks")
        .fetch_one(&pool).await
    {
        Ok(row) => row.get::<i64, _>("count") as usize,
        Err(_) => 0,
    };
    
    // Check last update date
    let last_update = match sqlx::query("SELECT value FROM metadata WHERE key = 'sp500_last_updated'")
        .fetch_optional(&pool).await
    {
        Ok(Some(row)) => row.get::<String, _>("value"),
        Ok(None) => "Never".to_string(),
        Err(_) => "Unknown".to_string(),
    };
    
    let status = if stock_count > 0 {
        format!("Initialized with {} stocks (Last update: {})", stock_count, last_update)
    } else {
        "Not initialized - click 'Initialize S&P 500 Stocks' to get started".to_string()
    };
    
    Ok(InitProgress {
        current_step: "Ready".to_string(),
        companies_processed: stock_count,
        total_companies: if stock_count > 0 { stock_count } else { 503 },
        status,
    })
}

#[tauri::command]
pub async fn check_database_schema() -> Result<String, String> {
    let pool = get_database_connection().await?;
    
    // Check if required tables exist
    let tables = vec!["stocks", "daily_prices", "metadata"];
    let mut missing_tables = Vec::new();
    
    for table in tables {
        let exists = sqlx::query(&format!("SELECT name FROM sqlite_master WHERE type='table' AND name='{}'", table))
            .fetch_optional(&pool).await
            .map_err(|e| format!("Database query error: {}", e))?;
            
        if exists.is_none() {
            missing_tables.push(table);
        }
    }
    
    if missing_tables.is_empty() {
        Ok("Database schema is ready".to_string())
    } else {
        Err(format!("Missing required tables: {}", missing_tables.join(", ")))
    }
}