use sqlx::{SqlitePool, Row};
use serde::{Deserialize, Serialize};
use crate::analysis::pe_statistics::{
    PEAnalysis, calculate_pe_statistics, calculate_value_score, 
    calculate_risk_score, is_value_stock, generate_reasoning
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockRecommendation {
    pub symbol: String,
    pub company_name: String,
    pub current_pe: Option<f64>,
    pub current_pe_date: Option<String>, // Date of the current P/E ratio
    pub value_score: f64,
    pub risk_score: f64,
    pub rank: usize,
    pub recommendation_type: String,
    pub reasoning: String,
    pub historical_min_pe: f64,
    pub historical_max_pe: f64,
    pub value_threshold: f64,
    pub data_points: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationStats {
    pub total_sp500_stocks: usize,
    pub stocks_with_pe_data: usize,
    pub value_stocks_found: usize,
    pub average_value_score: f64,
    pub average_risk_score: f64,
    pub top_10_symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationResponse {
    pub recommendations: Vec<StockRecommendation>,
    pub stats: RecommendationStats,
}

pub struct RecommendationEngine {
    pool: SqlitePool,
}

impl RecommendationEngine {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Analyze P/E ratios for all S&P 500 stocks (OPTIMIZED)
    pub async fn analyze_sp500_pe_values(&self) -> Result<Vec<PEAnalysis>, Box<dyn std::error::Error>> {
        println!("🔍 Starting optimized S&P 500 P/E analysis...");

        // Get all S&P 500 stocks that have P/E data
        let sp500_stocks = self.get_sp500_stocks_with_pe_data().await?;
        println!("📊 Found {} S&P 500 stocks with P/E data", sp500_stocks.len());

        // Use bulk analysis for better performance
        let analyses = self.bulk_analyze_stocks(sp500_stocks).await?;

        println!("✅ Completed P/E analysis for {} stocks", analyses.len());
        Ok(analyses)
    }

    /// Bulk analyze stocks with optimized database queries
    async fn bulk_analyze_stocks(&self, stocks: Vec<(i64, String, String)>) -> Result<Vec<PEAnalysis>, Box<dyn std::error::Error>> {
        use futures::future::join_all;
        use std::sync::Arc;
        
        // Create semaphore to limit concurrent database connections
        let semaphore = Arc::new(tokio::sync::Semaphore::new(20)); // Max 20 concurrent
        let pool = Arc::new(self.pool.clone());
        
        // Create concurrent tasks for each stock
        let tasks: Vec<_> = stocks.into_iter().map(|(stock_id, symbol, company_name)| {
            let semaphore = semaphore.clone();
            let pool = pool.clone();
            
            tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                // Perform bulk analysis for this stock
                match Self::analyze_single_stock_optimized(pool.as_ref(), stock_id, &symbol, &company_name).await {
                    Ok(analysis) => Some(analysis),
                    Err(e) => {
                        eprintln!("⚠️  Error analyzing {}: {}", symbol, e);
                        None
                    }
                }
            })
        }).collect();
        
        // Wait for all tasks to complete
        let results = join_all(tasks).await;
        
        // Collect successful results
        let mut analyses = Vec::new();
        for result in results {
            if let Ok(Some(analysis)) = result {
                analyses.push(analysis);
            }
        }
        
        Ok(analyses)
    }

    /// Optimized single stock analysis with fewer database calls
    async fn analyze_single_stock_optimized(
        pool: &SqlitePool,
        stock_id: i64,
        symbol: &str,
        company_name: &str
    ) -> Result<PEAnalysis, Box<dyn std::error::Error>> {
        // Single query to get all P/E data AND current P/E with date
        let query = "
            SELECT pe_ratio, date,
                   (SELECT pe_ratio FROM daily_prices dp2 
                    WHERE dp2.stock_id = ? AND dp2.pe_ratio IS NOT NULL 
                    ORDER BY dp2.date DESC LIMIT 1) as current_pe,
                   (SELECT date FROM daily_prices dp3 
                    WHERE dp3.stock_id = ? AND dp3.pe_ratio IS NOT NULL 
                    ORDER BY dp3.date DESC LIMIT 1) as current_date
            FROM daily_prices
            WHERE stock_id = ? AND pe_ratio IS NOT NULL AND pe_ratio > 0
            ORDER BY date
        ";

        let rows = sqlx::query(query)
            .bind(stock_id)
            .bind(stock_id)
            .bind(stock_id)
            .fetch_all(pool)
            .await?;

        if rows.is_empty() {
            return Ok(PEAnalysis {
                symbol: symbol.to_string(),
                company_name: company_name.to_string(),
                current_pe: None,
                current_pe_date: None,
                historical_min: 0.0,
                historical_max: 0.0,
                historical_avg: 0.0,
                historical_median: 0.0,
                value_score: 0.0,
                risk_score: 100.0,
                value_threshold: 0.0,
                is_value_stock: false,
                data_points: 0,
                reasoning: "No P/E data available".to_string(),
            });
        }

        // Extract P/E data and current values
        let pe_data: Vec<f64> = rows.iter().map(|r| r.get::<f64, _>("pe_ratio")).collect();
        let current_pe: Option<f64> = rows.first().and_then(|r| r.try_get("current_pe").ok());
        let current_pe_date: Option<String> = rows.first().and_then(|r| r.try_get("current_date").ok());

        // Calculate statistics
        let stats = calculate_pe_statistics(&pe_data);
        
        // Calculate scores
        let value_score = calculate_value_score(current_pe, &stats);
        let risk_score = calculate_risk_score(current_pe, &stats);
        let is_value = is_value_stock(current_pe, &stats);
        let value_threshold = stats.min * 1.20;

        let mut analysis = PEAnalysis {
            symbol: symbol.to_string(),
            company_name: company_name.to_string(),
            current_pe,
            current_pe_date,
            historical_min: stats.min,
            historical_max: stats.max,
            historical_avg: stats.mean,
            historical_median: stats.median,
            value_score,
            risk_score,
            value_threshold,
            is_value_stock: is_value,
            data_points: stats.data_points,
            reasoning: String::new(),
        };

        analysis.reasoning = generate_reasoning(&analysis);
        Ok(analysis)
    }

    /// Get value stock recommendations with stats in one optimized call
    pub async fn get_value_recommendations_with_stats(&self, limit: Option<usize>) -> Result<RecommendationResponse, Box<dyn std::error::Error>> {
        println!("🎯 Generating value stock recommendations with stats...");

        let analyses = self.analyze_sp500_pe_values().await?;
        
        // Calculate stats from all analyses
        let total_sp500 = self.count_sp500_stocks().await?;
        let stocks_with_pe = analyses.len();
        
        // Filter for value stocks and sort by value score
        let mut value_stocks: Vec<PEAnalysis> = analyses
            .into_iter()
            .filter(|analysis| analysis.is_value_stock && analysis.current_pe.is_some())
            .collect();

        // Sort by value score (descending) and risk score (ascending)
        value_stocks.sort_by(|a, b| {
            let score_cmp = b.value_score.partial_cmp(&a.value_score).unwrap();
            if score_cmp == std::cmp::Ordering::Equal {
                a.risk_score.partial_cmp(&b.risk_score).unwrap()
            } else {
                score_cmp
            }
        });

        // Calculate stats from value stocks
        let value_stocks_found = value_stocks.len();
        let avg_value_score = if !value_stocks.is_empty() {
            value_stocks.iter().map(|r| r.value_score).sum::<f64>() / value_stocks.len() as f64
        } else {
            0.0
        };
        let avg_risk_score = if !value_stocks.is_empty() {
            value_stocks.iter().map(|r| r.risk_score).sum::<f64>() / value_stocks.len() as f64
        } else {
            0.0
        };

        // Apply limit if specified
        let display_stocks = if let Some(limit) = limit {
            value_stocks.into_iter().take(limit).collect()
        } else {
            value_stocks
        };

        let top_10_symbols: Vec<String> = display_stocks
            .iter()
            .take(10)
            .map(|r| r.symbol.clone())
            .collect();

        // Convert to recommendations with ranking
        let recommendations: Vec<StockRecommendation> = display_stocks
            .into_iter()
            .enumerate()
            .map(|(index, analysis)| StockRecommendation {
                symbol: analysis.symbol.clone(),
                company_name: analysis.company_name.clone(),
                current_pe: analysis.current_pe,
                current_pe_date: analysis.current_pe_date.clone(),
                value_score: analysis.value_score,
                risk_score: analysis.risk_score,
                rank: index + 1,
                recommendation_type: "Value Investment".to_string(),
                reasoning: analysis.reasoning.clone(),
                historical_min_pe: analysis.historical_min,
                historical_max_pe: analysis.historical_max,
                value_threshold: analysis.value_threshold,
                data_points: analysis.data_points,
            })
            .collect();

        let stats = RecommendationStats {
            total_sp500_stocks: total_sp500,
            stocks_with_pe_data: stocks_with_pe,
            value_stocks_found,
            average_value_score: avg_value_score,
            average_risk_score: avg_risk_score,
            top_10_symbols,
        };

        println!("✅ Generated {} value stock recommendations with stats", recommendations.len());
        Ok(RecommendationResponse {
            recommendations,
            stats,
        })
    }

    /// Get value stock recommendations based on P/E criteria (legacy method)
    pub async fn get_value_recommendations(&self, limit: Option<usize>) -> Result<Vec<StockRecommendation>, Box<dyn std::error::Error>> {
        let response = self.get_value_recommendations_with_stats(limit).await?;
        Ok(response.recommendations)
    }

    /// Analyze P/E history for a specific stock
    pub async fn analyze_stock_pe_history(&self, stock_id: i64, symbol: &str, company_name: &str) -> Result<PEAnalysis, Box<dyn std::error::Error>> {
        // Get all P/E data for this stock
        let pe_data = self.get_stock_pe_data(stock_id).await?;
        
        if pe_data.is_empty() {
            return Ok(PEAnalysis {
                symbol: symbol.to_string(),
                company_name: company_name.to_string(),
                current_pe: None,
                current_pe_date: None,
                historical_min: 0.0,
                historical_max: 0.0,
                historical_avg: 0.0,
                historical_median: 0.0,
                value_score: 0.0,
                risk_score: 100.0,
                value_threshold: 0.0,
                is_value_stock: false,
                data_points: 0,
                reasoning: "No P/E data available".to_string(),
            });
        }

        // Calculate statistics
        let stats = calculate_pe_statistics(&pe_data);
        
        // Get current (most recent) P/E ratio with date
        let (current_pe, current_pe_date) = match self.get_current_pe_with_date(stock_id).await? {
            Some((pe, date)) => (Some(pe), Some(date)),
            None => (None, None),
        };
        
        // Calculate scores
        let value_score = calculate_value_score(current_pe, &stats);
        let risk_score = calculate_risk_score(current_pe, &stats);
        let is_value = is_value_stock(current_pe, &stats);
        let value_threshold = stats.min * 1.20;

        let mut analysis = PEAnalysis {
            symbol: symbol.to_string(),
            company_name: company_name.to_string(),
            current_pe,
            current_pe_date,
            historical_min: stats.min,
            historical_max: stats.max,
            historical_avg: stats.mean,
            historical_median: stats.median,
            value_score,
            risk_score,
            value_threshold,
            is_value_stock: is_value,
            data_points: stats.data_points,
            reasoning: String::new(),
        };

        analysis.reasoning = generate_reasoning(&analysis);

        Ok(analysis)
    }

    /// Get recommendation statistics
    pub async fn get_recommendation_stats(&self) -> Result<RecommendationStats, Box<dyn std::error::Error>> {
        let recommendations = self.get_value_recommendations(None).await?;
        
        let total_sp500 = self.count_sp500_stocks().await?;
        let stocks_with_pe = self.count_sp500_stocks_with_pe().await?;
        let value_stocks = recommendations.len();
        
        let avg_value_score = if !recommendations.is_empty() {
            recommendations.iter().map(|r| r.value_score).sum::<f64>() / recommendations.len() as f64
        } else {
            0.0
        };

        let avg_risk_score = if !recommendations.is_empty() {
            recommendations.iter().map(|r| r.risk_score).sum::<f64>() / recommendations.len() as f64
        } else {
            0.0
        };

        let top_10_symbols: Vec<String> = recommendations
            .iter()
            .take(10)
            .map(|r| r.symbol.clone())
            .collect();

        Ok(RecommendationStats {
            total_sp500_stocks: total_sp500,
            stocks_with_pe_data: stocks_with_pe,
            value_stocks_found: value_stocks,
            average_value_score: avg_value_score,
            average_risk_score: avg_risk_score,
            top_10_symbols,
        })
    }

    /// Get S&P 500 stocks that have P/E data (OPTIMIZED with cache table)
    async fn get_sp500_stocks_with_pe_data(&self) -> Result<Vec<(i64, String, String)>, Box<dyn std::error::Error>> {
        let query = "SELECT id, symbol, company_name FROM sp500_pe_cache ORDER BY symbol";
        let rows = sqlx::query(query).fetch_all(&self.pool).await?;

        let stocks = rows
            .into_iter()
            .map(|row| {
                let id: i64 = row.get("id");
                let symbol: String = row.get("symbol");
                let company_name: String = row.get("company_name");
                (id, symbol, company_name)
            })
            .collect();

        Ok(stocks)
    }

    /// Get all P/E ratios for a specific stock
    async fn get_stock_pe_data(&self, stock_id: i64) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
        let query = "
            SELECT pe_ratio
            FROM daily_prices
            WHERE stock_id = ? AND pe_ratio IS NOT NULL AND pe_ratio > 0
            ORDER BY date
        ";

        let rows = sqlx::query(query)
            .bind(stock_id)
            .fetch_all(&self.pool)
            .await?;

        let pe_data: Vec<f64> = rows
            .into_iter()
            .map(|row| row.get::<f64, _>("pe_ratio"))
            .collect();

        Ok(pe_data)
    }

    /// Get the most recent P/E ratio for a stock
    async fn get_current_pe_ratio(&self, stock_id: i64) -> Result<Option<f64>, Box<dyn std::error::Error>> {
        let query = "
            SELECT pe_ratio
            FROM daily_prices
            WHERE stock_id = ? AND pe_ratio IS NOT NULL
            ORDER BY date DESC
            LIMIT 1
        ";

        let row = sqlx::query(query)
            .bind(stock_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.get::<f64, _>("pe_ratio")))
    }

    /// Get the most recent P/E ratio with date for a stock
    async fn get_current_pe_with_date(&self, stock_id: i64) -> Result<Option<(f64, String)>, Box<dyn std::error::Error>> {
        let query = "
            SELECT pe_ratio, date
            FROM daily_prices
            WHERE stock_id = ? AND pe_ratio IS NOT NULL
            ORDER BY date DESC
            LIMIT 1
        ";

        let row = sqlx::query(query)
            .bind(stock_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| {
            let pe_ratio: f64 = r.get("pe_ratio");
            let date: String = r.get("date");
            (pe_ratio, date)
        }))
    }

    /// Count total S&P 500 stocks
    async fn count_sp500_stocks(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let query = "SELECT COUNT(*) as count FROM sp500_symbols";
        let row = sqlx::query(query).fetch_one(&self.pool).await?;
        Ok(row.get::<i64, _>("count") as usize)
    }

    /// Count S&P 500 stocks with P/E data (OPTIMIZED)
    async fn count_sp500_stocks_with_pe(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let query = "SELECT COUNT(*) as count FROM sp500_pe_cache";
        let row = sqlx::query(query).fetch_one(&self.pool).await?;
        Ok(row.get::<i64, _>("count") as usize)
    }
}