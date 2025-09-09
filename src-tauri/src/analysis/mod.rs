use anyhow::Result;
use chrono::{Duration, NaiveDate, Utc};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use tracing::{info, warn};

use crate::database_sqlx::DatabaseManagerSqlx;
use crate::models::{Stock, StockAnalysis, StockDetail, DailyPrice, DatabaseStats};

pub mod pe_statistics;
pub mod recommendation_engine;

pub use pe_statistics::*;
pub use recommendation_engine::*;

// Re-export Tauri commands from commands::analysis
pub use crate::commands::analysis::{
    get_undervalued_stocks_by_ps
};

#[allow(dead_code)]
pub struct AnalysisEngine {
    database: DatabaseManagerSqlx,
    fuzzy_matcher: SkimMatcherV2,
}

impl AnalysisEngine {
    /// Create a new analysis engine
    #[allow(dead_code)]
    pub fn new(database: DatabaseManagerSqlx) -> Self {
        Self {
            database,
            fuzzy_matcher: SkimMatcherV2::default(),
        }
    }

    /// Get top stocks with maximum P/E ratio decline over the last year
    #[allow(dead_code)]
    pub async fn get_top_pe_decliners(&self, limit: usize, offset: usize) -> Result<Vec<StockAnalysis>> {
        info!("Calculating P/E decliners with limit={}, offset={}", limit, offset);
        
        let stocks = self.database.get_active_stocks().await?;
        let mut analyses = Vec::new();

        let one_year_ago = Utc::now().date_naive() - Duration::days(365);

        for stock in stocks {
            if let Some(stock_id) = stock.id {
                if let Some(analysis) = self.calculate_pe_analysis(stock_id, stock, one_year_ago).await? {
                    if analysis.pe_decline_percent > 0.0 {  // Only include stocks with actual P/E decline
                        analyses.push(analysis);
                    }
                }
            }
        }

        // Sort by P/E decline percentage (descending)
        analyses.sort_by(|a, b| b.pe_decline_percent.partial_cmp(&a.pe_decline_percent).unwrap_or(std::cmp::Ordering::Equal));

        // Apply pagination
        let end = std::cmp::min(offset + limit, analyses.len());
        let start = std::cmp::min(offset, analyses.len());
        
        info!("Found {} stocks with P/E decline, returning {} results", analyses.len(), end - start);
        Ok(analyses[start..end].to_vec())
    }

    /// Calculate P/E analysis for a single stock
    #[allow(dead_code)]
    async fn calculate_pe_analysis(
        &self, 
        stock_id: i64, 
        stock: Stock, 
        one_year_ago: NaiveDate
    ) -> Result<Option<StockAnalysis>> {
        // Get current price data
        let current_price_data = match self.database.get_latest_price(stock_id).await? {
            Some(price) => price,
            None => {
                warn!("No current price data for stock: {}", stock.symbol);
                return Ok(None);
            }
        };

        // Get price data from one year ago (or closest available)
        let year_ago_price_data = self.get_price_near_date(stock_id, one_year_ago).await?;
        
        let year_ago_price_data = match year_ago_price_data {
            Some(price) => price,
            None => {
                warn!("No historical price data for stock: {} around {}", stock.symbol, one_year_ago);
                return Ok(None);
            }
        };

        // Calculate P/E decline
        let pe_decline_percent = match (year_ago_price_data.pe_ratio, current_price_data.pe_ratio) {
            (Some(year_ago_pe), Some(current_pe)) if year_ago_pe > 0.0 => {
                ((year_ago_pe - current_pe) / year_ago_pe) * 100.0
            }
            _ => 0.0, // Can't calculate P/E decline without valid P/E ratios
        };

        // Calculate price change
        let price_change_percent = ((current_price_data.close_price - year_ago_price_data.close_price) 
                                   / year_ago_price_data.close_price) * 100.0;

        Ok(Some(StockAnalysis {
            stock,
            current_price: current_price_data.close_price,
            current_pe: current_price_data.pe_ratio,
            year_ago_pe: year_ago_price_data.pe_ratio,
            pe_decline_percent,
            price_change_percent,
        }))
    }

    /// Get price data near a specific date
    #[allow(dead_code)]
    async fn get_price_near_date(&self, stock_id: i64, target_date: NaiveDate) -> Result<Option<DailyPrice>> {
        // First try exact date
        if let Some(price) = self.database.get_price_on_date(stock_id, target_date).await? {
            return Ok(Some(price));
        }

        // If no exact match, search within a 30-day window around the target date
        for days_offset in 1..=30 {
            // Try earlier dates first
            let earlier_date = target_date - Duration::days(days_offset);
            if let Some(price) = self.database.get_price_on_date(stock_id, earlier_date).await? {
                return Ok(Some(price));
            }

            // Then try later dates
            let later_date = target_date + Duration::days(days_offset);
            if let Some(price) = self.database.get_price_on_date(stock_id, later_date).await? {
                return Ok(Some(price));
            }
        }

        Ok(None)
    }

    /// Search for stocks by symbol or company name
    #[allow(dead_code)]
    pub async fn search_stocks(&self, query: &str) -> Result<Vec<Stock>> {
        info!("Searching stocks with query: '{}'", query);
        
        let all_stocks = self.database.get_active_stocks().await?;
        let query_lower = query.to_lowercase();
        
        let mut results = Vec::new();
        let mut scored_results = Vec::new();

        for stock in all_stocks {
            let mut score = 0i64;
            let mut exact_match = false;

            // Check for exact symbol match (highest priority)
            if stock.symbol.to_lowercase() == query_lower {
                score = 1000;
                exact_match = true;
            }
            // Check for symbol prefix match
            else if stock.symbol.to_lowercase().starts_with(&query_lower) {
                score = 800;
            }
            // Check for symbol substring match
            else if stock.symbol.to_lowercase().contains(&query_lower) {
                score = 600;
            }
            // Use fuzzy matching on company name
            else if let Some(fuzzy_score) = self.fuzzy_matcher.fuzzy_match(&stock.company_name.to_lowercase(), &query_lower) {
                score = fuzzy_score;
            }

            if score > 0 {
                scored_results.push((stock, score, exact_match));
            }
        }

        // Sort by score (descending), with exact matches first
        scored_results.sort_by(|a, b| {
            match (a.2, b.2) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.1.cmp(&a.1),
            }
        });

        // Extract just the stocks, limiting to top 20 results
        for (stock, _, _) in scored_results.into_iter().take(20) {
            results.push(stock);
        }

        info!("Search returned {} results", results.len());
        Ok(results)
    }

    /// Get detailed information for a specific stock
    #[allow(dead_code)]
    pub async fn get_stock_details(&self, symbol: &str) -> Result<Option<StockDetail>> {
        info!("Getting stock details for: {}", symbol);
        
        let stock = match self.database.get_stock_by_symbol(symbol).await? {
            Some(stock) => stock,
            None => return Ok(None),
        };

        let stock_id = match stock.id {
            Some(id) => id,
            None => return Ok(None),
        };

        // Get current price
        let current_price = match self.database.get_latest_price(stock_id).await? {
            Some(price) => price,
            None => return Ok(None),
        };

        // Get price history for the last year
        let one_year_ago = Utc::now().date_naive() - Duration::days(365);
        let price_history = self.get_price_history_range(stock_id, one_year_ago, Utc::now().date_naive()).await?;

        // Extract P/E trend data
        let pe_trend: Vec<(NaiveDate, f64)> = price_history
            .iter()
            .filter_map(|p| p.pe_ratio.map(|pe| (p.date, pe)))
            .collect();

        // Extract volume trend data
        let volume_trend: Vec<(NaiveDate, i64)> = price_history
            .iter()
            .filter_map(|p| p.volume.map(|vol| (p.date, vol)))
            .collect();

        Ok(Some(StockDetail {
            stock,
            current_price,
            price_history,
            pe_trend,
            volume_trend,
        }))
    }

    /// Get price history within a date range
    #[allow(dead_code)]
    async fn get_price_history_range(&self, stock_id: i64, from_date: NaiveDate, to_date: NaiveDate) -> Result<Vec<DailyPrice>> {
        // This is a simplified implementation - in a real scenario you'd want a more efficient query
        let mut history = Vec::new();
        let mut current_date = from_date;

        while current_date <= to_date {
            if let Some(price) = self.database.get_price_on_date(stock_id, current_date).await? {
                history.push(price);
            }
            current_date = current_date + Duration::days(1);
        }

        Ok(history)
    }

    /// Get comprehensive database statistics for analysis
    #[allow(dead_code)]
    pub async fn get_database_stats(&self) -> Result<DatabaseStats> {
        let stats = self.database.get_stats().await?;
        
        // Get top P/E decliner
        let top_decliners = self.get_top_pe_decliners(1, 0).await?;
        let top_pe_decliner = top_decliners.first().map(|analysis| {
            (analysis.stock.symbol.clone(), analysis.pe_decline_percent)
        });
        
        let total_stocks = stats.get("total_stocks").unwrap_or(&0).clone() as usize;
        let total_price_records = stats.get("total_prices").unwrap_or(&0).clone() as usize;
        
        // Calculate coverage percentage
        let data_coverage_percentage = if total_stocks > 0 {
            (total_price_records as f64 / (total_stocks * 1000) as f64) * 100.0 // Rough estimate
        } else {
            0.0
        };

        Ok(DatabaseStats {
            total_stocks,
            total_price_records,
            data_coverage_percentage,
            oldest_data_date: self.database.get_oldest_data_date().await.unwrap_or(None),
            last_update_date: self.database.get_last_update_date().await.unwrap_or(None),
            top_pe_decliner,
        })
    }
}

// SummaryStats consolidated into DatabaseStats in models/mod.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database_sqlx::DatabaseManagerSqlx;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_search_stocks() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = DatabaseManagerSqlx::new(db_path.to_str().unwrap()).await.unwrap();
        
        // Insert test stock
        let stock = Stock {
            id: None,
            symbol: "AAPL".to_string(),
            company_name: "Apple Inc.".to_string(),
            sector: Some("Technology".to_string()),
            industry: None,
            market_cap: Some(3000000000000.0),
            status: crate::models::StockStatus::Active,
            first_trading_date: None,
            last_updated: None,
        };
        
        db.upsert_stock(&stock).await.unwrap();
        
        let analysis_engine = AnalysisEngine::new(db);
        
        // Test exact symbol match
        let results = analysis_engine.search_stocks("AAPL").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol, "AAPL");
        
        // Test company name fuzzy match
        let results = analysis_engine.search_stocks("Apple").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].company_name, "Apple Inc.");
    }
}