use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInfo {
    pub id: i64,
    pub symbol: String,
    pub company_name: String,
    pub sector: Option<String>,
}

#[tauri::command]
pub async fn get_all_stocks() -> Result<Vec<StockInfo>, String> {
    // TODO: Implement database connection
    // For now, return dummy data to test the UI
    Ok(vec![
        StockInfo {
            id: 1,
            symbol: "AAPL".to_string(),
            company_name: "Apple Inc.".to_string(),
            sector: Some("Technology".to_string()),
        },
        StockInfo {
            id: 2,
            symbol: "MSFT".to_string(),
            company_name: "Microsoft Corporation".to_string(),
            sector: Some("Technology".to_string()),
        },
        StockInfo {
            id: 3,
            symbol: "GOOGL".to_string(),
            company_name: "Alphabet Inc.".to_string(),
            sector: Some("Technology".to_string()),
        },
    ])
}

#[tauri::command]
pub async fn search_stocks(query: String) -> Result<Vec<StockInfo>, String> {
    // TODO: Implement fuzzy search
    // For now, filter dummy data
    let all_stocks = get_all_stocks().await?;
    let filtered = all_stocks.into_iter()
        .filter(|stock| 
            stock.symbol.to_lowercase().contains(&query.to_lowercase()) ||
            stock.company_name.to_lowercase().contains(&query.to_lowercase())
        )
        .collect();
    Ok(filtered)
}