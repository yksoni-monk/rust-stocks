use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PEStatistics {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub percentile_25: f64,
    pub percentile_75: f64,
    pub volatility: f64,
    pub data_points: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PEAnalysis {
    pub symbol: String,
    pub company_name: String,
    pub current_pe: Option<f64>,
    pub current_pe_date: Option<String>, // Date of the current P/E ratio
    pub historical_min: f64,
    pub historical_max: f64,
    pub historical_avg: f64,
    pub historical_median: f64,
    pub value_score: f64,
    pub risk_score: f64,
    pub value_threshold: f64, // 20% above historical min
    pub is_value_stock: bool,
    pub data_points: usize,
    pub reasoning: String,
}

impl PEStatistics {
    pub fn new() -> Self {
        Self {
            min: 0.0,
            max: 0.0,
            mean: 0.0,
            median: 0.0,
            percentile_25: 0.0,
            percentile_75: 0.0,
            volatility: 0.0,
            data_points: 0,
        }
    }
}

/// Calculate comprehensive P/E statistics from historical data
pub fn calculate_pe_statistics(pe_data: &[f64]) -> PEStatistics {
    if pe_data.is_empty() {
        return PEStatistics::new();
    }

    // Filter out negative P/E ratios for statistical analysis
    let positive_pe: Vec<f64> = pe_data.iter().copied().filter(|&pe| pe > 0.0).collect();
    
    if positive_pe.is_empty() {
        return PEStatistics::new();
    }

    let mut sorted_pe = positive_pe.clone();
    sorted_pe.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let len = sorted_pe.len();
    let min = sorted_pe[0];
    let max = sorted_pe[len - 1];
    let mean = sorted_pe.iter().sum::<f64>() / len as f64;
    
    // Calculate median
    let median = if len % 2 == 0 {
        (sorted_pe[len / 2 - 1] + sorted_pe[len / 2]) / 2.0
    } else {
        sorted_pe[len / 2]
    };

    // Calculate percentiles
    let percentile_25_idx = (len as f64 * 0.25) as usize;
    let percentile_75_idx = (len as f64 * 0.75) as usize;
    let percentile_25 = sorted_pe[percentile_25_idx.min(len - 1)];
    let percentile_75 = sorted_pe[percentile_75_idx.min(len - 1)];

    // Calculate volatility (standard deviation)
    let variance = sorted_pe.iter()
        .map(|&pe| (pe - mean).powi(2))
        .sum::<f64>() / len as f64;
    let volatility = variance.sqrt();

    PEStatistics {
        min,
        max,
        mean,
        median,
        percentile_25,
        percentile_75,
        volatility,
        data_points: len,
    }
}

/// Calculate value score based on current P/E position relative to historical range
pub fn calculate_value_score(current_pe: Option<f64>, stats: &PEStatistics) -> f64 {
    let Some(current) = current_pe else {
        return 0.0; // No current P/E data
    };

    if current <= 0.0 || stats.data_points == 0 || stats.max <= stats.min {
        return 0.0; // Invalid data
    }

    // Base score: Position in historical range (inverted - lower P/E = higher score)
    let range = stats.max - stats.min;
    let position_from_min = (current - stats.min) / range;
    let base_score = (1.0 - position_from_min) * 100.0;

    // Bonus for being near historical minimum
    let near_min_bonus = if current <= stats.min * 1.1 {
        20.0 // 20 point bonus for being within 10% of historical minimum
    } else if current <= stats.min * 1.2 {
        10.0 // 10 point bonus for being within 20% of historical minimum
    } else {
        0.0
    };

    // Penalty for high volatility (risky stocks)
    let volatility_penalty = if stats.volatility > stats.mean * 0.5 {
        -10.0 // High volatility penalty
    } else if stats.volatility > stats.mean * 0.3 {
        -5.0 // Moderate volatility penalty
    } else {
        0.0
    };

    // Ensure score is between 0 and 120
    (base_score + near_min_bonus + volatility_penalty).max(0.0).min(120.0)
}

/// Calculate risk score based on P/E volatility and current position
pub fn calculate_risk_score(current_pe: Option<f64>, stats: &PEStatistics) -> f64 {
    let Some(current) = current_pe else {
        return 100.0; // Maximum risk if no current data
    };

    if current <= 0.0 || stats.data_points == 0 {
        return 100.0; // Maximum risk for negative P/E or no data
    }

    let mut risk_score = 0.0;

    // Volatility risk (0-40 points)
    let volatility_risk = (stats.volatility / stats.mean * 40.0).min(40.0);
    risk_score += volatility_risk;

    // Extreme P/E risk (0-30 points)
    let extreme_pe_risk = if current > stats.mean * 2.0 {
        30.0 // Very high P/E compared to historical average
    } else if current > stats.percentile_75 * 1.5 {
        20.0 // High P/E compared to 75th percentile
    } else if current < stats.percentile_25 * 0.5 {
        15.0 // Unusually low P/E might indicate problems
    } else {
        0.0
    };
    risk_score += extreme_pe_risk;

    // Data quality risk (0-30 points)
    let data_quality_risk = if stats.data_points < 50 {
        30.0 // Less than ~3 months of data
    } else if stats.data_points < 100 {
        20.0 // Less than ~6 months of data
    } else if stats.data_points < 250 {
        10.0 // Less than ~1 year of data
    } else {
        0.0
    };
    risk_score += data_quality_risk;

    risk_score.min(100.0)
}

/// Check if stock qualifies as a value investment based on P/E criteria
pub fn is_value_stock(current_pe: Option<f64>, stats: &PEStatistics) -> bool {
    let Some(current) = current_pe else {
        return false; // No current P/E data
    };

    if current <= 0.0 || stats.data_points < 100 {
        return false; // Exclude negative P/E or insufficient data
    }

    // Value criteria: Current P/E ≤ Historical Min × 1.20 (20% above historical low)
    let value_threshold = stats.min * 1.20;
    current <= value_threshold
}

/// Generate human-readable reasoning for the recommendation
pub fn generate_reasoning(analysis: &PEAnalysis) -> String {
    let mut reasons = Vec::new();

    if let Some(current_pe) = analysis.current_pe {
        if analysis.is_value_stock {
            let pct_above_min = ((current_pe / analysis.historical_min) - 1.0) * 100.0;
            let date_str = analysis.current_pe_date.as_ref()
                .map(|d| format!(" (as of {})", d))
                .unwrap_or_default();
            reasons.push(format!(
                "P/E of {:.1}{} is only {:.1}% above historical minimum of {:.1}",
                current_pe, date_str, pct_above_min, analysis.historical_min
            ));
        }

        if current_pe < analysis.historical_median {
            reasons.push(format!(
                "Current P/E ({:.1}) is below historical median ({:.1})",
                current_pe, analysis.historical_median
            ));
        }

        if analysis.value_score > 80.0 {
            reasons.push("High value score indicates strong relative valuation".to_string());
        } else if analysis.value_score > 60.0 {
            reasons.push("Moderate value score shows reasonable valuation".to_string());
        }

        if analysis.risk_score < 30.0 {
            reasons.push("Low risk profile with stable P/E history".to_string());
        } else if analysis.risk_score > 70.0 {
            reasons.push("Higher risk due to P/E volatility or extreme values".to_string());
        }
    } else {
        reasons.push("No current P/E data available for analysis".to_string());
    }

    if analysis.data_points < 100 {
        reasons.push("Limited historical data - use caution".to_string());
    }

    if reasons.is_empty() {
        "Standard analysis based on available P/E data".to_string()
    } else {
        reasons.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pe_statistics() {
        let pe_data = vec![10.0, 15.0, 20.0, 25.0, 30.0];
        let stats = calculate_pe_statistics(&pe_data);
        
        assert_eq!(stats.min, 10.0);
        assert_eq!(stats.max, 30.0);
        assert_eq!(stats.mean, 20.0);
        assert_eq!(stats.median, 20.0);
        assert_eq!(stats.data_points, 5);
    }

    #[test]
    fn test_value_scoring() {
        let stats = PEStatistics {
            min: 10.0,
            max: 30.0,
            mean: 20.0,
            median: 20.0,
            percentile_25: 15.0,
            percentile_75: 25.0,
            volatility: 5.0,
            data_points: 100,
        };

        // Low P/E should get high value score
        let low_pe_score = calculate_value_score(Some(12.0), &stats);
        assert!(low_pe_score > 80.0);

        // High P/E should get low value score
        let high_pe_score = calculate_value_score(Some(28.0), &stats);
        assert!(high_pe_score < 30.0);
    }

    #[test]
    fn test_value_stock_criteria() {
        let stats = PEStatistics {
            min: 10.0,
            max: 30.0,
            mean: 20.0,
            median: 20.0,
            percentile_25: 15.0,
            percentile_75: 25.0,
            volatility: 5.0,
            data_points: 100,
        };

        // Should qualify: 12.0 ≤ (10.0 × 1.20 = 12.0)
        assert!(is_value_stock(Some(12.0), &stats));

        // Should not qualify: 13.0 > (10.0 × 1.20 = 12.0)
        assert!(!is_value_stock(Some(13.0), &stats));

        // Should not qualify: negative P/E
        assert!(!is_value_stock(Some(-5.0), &stats));
    }
}