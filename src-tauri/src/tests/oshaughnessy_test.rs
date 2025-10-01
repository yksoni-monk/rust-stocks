use crate::commands::oshaughnessy_screening::get_oshaughnessy_screening_results;

#[tokio::test]
async fn test_oshaughnessy_api_basic() {
    println!("🧪 Testing O'Shaughnessy API...");

    // Test with empty stock list (should return from database)
    let result = get_oshaughnessy_screening_results(vec![], None, Some(5)).await;

    match result {
        Ok(stocks) => {
            println!("✅ Success! Got {} stocks", stocks.len());
            if !stocks.is_empty() {
                println!("📊 First stock: {:?}", stocks[0].symbol);
                println!("📊 Composite score: {}", stocks[0].composite_score);
            }
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            panic!("O'Shaughnessy API failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_oshaughnessy_with_criteria() {
    println!("🧪 Testing O'Shaughnessy API with criteria...");

    use crate::commands::oshaughnessy_screening::OShaughnessyScreeningCriteria;

    let criteria = OShaughnessyScreeningCriteria {
        max_composite_percentile: Some(50.0),
        max_ps_ratio: Some(5.0),
        max_evs_ratio: Some(5.0),
        min_market_cap: Some(100_000_000.0),
        sectors: None,
        passes_screening_only: Some(false),
    };

    let result = get_oshaughnessy_screening_results(vec![], Some(criteria), Some(10)).await;

    match result {
        Ok(stocks) => {
            println!("✅ Success with criteria! Got {} stocks", stocks.len());
            for stock in stocks.iter().take(3) {
                println!("📊 {}: P/S={:?}, Composite={}, Percentile={}",
                    stock.symbol,
                    stock.ps_ratio,
                    stock.composite_score,
                    stock.composite_percentile
                );
            }
        }
        Err(e) => {
            println!("❌ Error with criteria: {}", e);
            panic!("O'Shaughnessy API with criteria failed: {}", e);
        }
    }
}