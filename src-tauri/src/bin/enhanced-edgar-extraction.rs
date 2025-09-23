// Enhanced EDGAR extraction to support multi-year data for Piotroski F-Score
// This modification changes the database insertion logic to store multiple years of data

// Replace the insert_financial_data_to_db function in concurrent-edgar-extraction.rs
async fn insert_financial_data_to_db(db_pool: &SqlitePool, data: &ExtractedFinancialData) -> Result<()> {
    let mut tx = db_pool.begin().await?;
    
    // Insert income statements - store multiple years instead of replacing
    for income_stmt in &data.income_statements {
        let period_type = match income_stmt.period.as_str() {
            "FY" => "TTM", // Use TTM for annual data to match Piotroski requirements
            "Q1" | "Q2" | "Q3" | "Q4" => "Quarterly",
            _ => "Quarterly", // Default to quarterly
        };
        
        let fiscal_period = if income_stmt.period == "FY" {
            None
        } else {
            Some(income_stmt.period.clone())
        };
        
        // Use INSERT OR IGNORE to avoid overwriting existing data
        // This allows us to store multiple years of data
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO income_statements 
            (stock_id, period_type, report_date, fiscal_year, fiscal_period, revenue, net_income, operating_income, shares_basic, shares_diluted, data_source)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'edgar')
            "#
        )
        .bind(income_stmt.stock_id)
        .bind(period_type)
        .bind(&income_stmt.end_date)
        .bind(income_stmt.year)
        .bind(fiscal_period)
        .bind(income_stmt.revenue)
        .bind(income_stmt.net_income)
        .bind(income_stmt.operating_income)
        .bind(income_stmt.shares_basic)
        .bind(income_stmt.shares_diluted)
        .execute(&mut *tx)
        .await?;
    }
    
    // Insert balance sheets - store multiple years instead of replacing
    for balance_sheet in &data.balance_sheets {
        let period_type = match balance_sheet.period.as_str() {
            "FY" => "TTM", // Use TTM for annual data to match Piotroski requirements
            "Q1" | "Q2" | "Q3" | "Q4" => "Quarterly",
            _ => "Quarterly", // Default to quarterly
        };
        
        let fiscal_period = if balance_sheet.period == "FY" {
            None
        } else {
            Some(balance_sheet.period.clone())
        };
        
        // Use INSERT OR IGNORE to avoid overwriting existing data
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO balance_sheets 
            (stock_id, period_type, report_date, fiscal_year, fiscal_period, total_assets, total_debt, total_equity, cash_and_equivalents, shares_outstanding, data_source)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'edgar')
            "#
        )
        .bind(balance_sheet.stock_id)
        .bind(period_type)
        .bind(&balance_sheet.end_date)
        .bind(balance_sheet.year)
        .bind(fiscal_period)
        .bind(balance_sheet.total_assets)
        .bind(balance_sheet.total_debt)
        .bind(balance_sheet.total_equity)
        .bind(balance_sheet.cash_and_equivalents)
        .bind(balance_sheet.shares_outstanding)
        .execute(&mut *tx)
        .await?;
    }
    
    // Insert cash flow statements - store multiple years instead of replacing
    for cash_flow_stmt in &data.cash_flow_statements {
        let period_type = match cash_flow_stmt.period.as_str() {
            "FY" => "TTM", // Use TTM for annual data to match Piotroski requirements
            "Q1" | "Q2" | "Q3" | "Q4" => "Quarterly",
            _ => "Quarterly", // Default to quarterly
        };
        
        let fiscal_period = if cash_flow_stmt.period == "FY" {
            None
        } else {
            Some(cash_flow_stmt.period.clone())
        };
        
        // Use INSERT OR IGNORE to avoid overwriting existing data
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO cash_flow_statements 
            (stock_id, period_type, report_date, fiscal_year, fiscal_period, operating_cash_flow, investing_cash_flow, financing_cash_flow, net_cash_flow, depreciation_expense, dividends_paid, share_repurchases, amortization_expense, data_source)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'edgar')
            "#
        )
        .bind(cash_flow_stmt.stock_id)
        .bind(period_type)
        .bind(&cash_flow_stmt.end_date)
        .bind(cash_flow_stmt.year)
        .bind(fiscal_period)
        .bind(cash_flow_stmt.operating_cash_flow)
        .bind(cash_flow_stmt.investing_cash_flow)
        .bind(cash_flow_stmt.financing_cash_flow)
        .bind(cash_flow_stmt.net_cash_flow)
        .bind(cash_flow_stmt.depreciation_expense)
        .bind(cash_flow_stmt.dividends_paid)
        .bind(cash_flow_stmt.share_repurchases)
        .bind(cash_flow_stmt.amortization_expense)
        .execute(&mut *tx)
        .await?;
    }
    
    tx.commit().await?;
    Ok(())
}

// Enhanced extraction logic to prioritize TTM data and capture multiple years
fn extract_available_periods(gaap_facts: &HashMap<String, EdgarConcept>) -> Result<Vec<PeriodInfo>> {
    let mut periods = Vec::new();
    let mut seen_periods = HashSet::new();
    
    // Look through all fields to find available periods
    for (_field_name, concept) in gaap_facts {
        if let Some(usd_values) = concept.units.get("USD") {
            for fact_value in usd_values {
                if let (Some(fy), Some(fp)) = (fact_value.fy, fact_value.fp.as_ref()) {
                    // Prioritize TTM (annual) data for Piotroski F-Score
                    let period_key = format!("{}-{}", fy, fp);
                    if !seen_periods.contains(&period_key) {
                        seen_periods.insert(period_key);
                        periods.push(PeriodInfo {
                            year: fy,
                            period: fp.clone(),
                            end_date: fact_value.end.clone(),
                        });
                    }
                }
            }
        }
    }
    
    // Sort periods by year descending (most recent first) and prioritize TTM data
    periods.sort_by(|a, b| {
        // First sort by year (descending)
        match b.year.cmp(&a.year) {
            std::cmp::Ordering::Equal => {
                // Within same year, prioritize TTM over quarterly
                match (a.period.as_str(), b.period.as_str()) {
                    ("FY", _) => std::cmp::Ordering::Less, // TTM comes first
                    (_, "FY") => std::cmp::Ordering::Greater,
                    _ => a.period.cmp(&b.period),
                }
            }
            other => other,
        }
    });
    
    // Limit to last 3 years to avoid excessive data storage
    periods.truncate(3);
    
    Ok(periods)
}
