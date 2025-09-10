use std::env;

/// Test configuration for database selection
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub use_production_db: bool,
    pub test_db_path: String,
}

impl TestConfig {
    /// Create test configuration from environment variables
    pub fn from_env() -> Self {
        // Check for environment variable to use production database
        let use_production_db = env::var("USE_PRODUCTION_DB")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(false);
        
        let test_db_path = env::var("TEST_DB_PATH")
            .unwrap_or_else(|_| "db/test.db".to_string());
        
        if use_production_db {
            println!("🚨 WARNING: Tests will run against PRODUCTION database");
            println!("   Production DB: db/stocks.db");
        } else {
            println!("🧪 Tests will run against test database: {}", test_db_path);
        }
        
        TestConfig {
            use_production_db,
            test_db_path,
        }
    }
    
    /// Get the database URL for testing
    pub fn get_database_url(&self) -> String {
        if self.use_production_db {
            "sqlite:db/stocks.db".to_string()
        } else {
            format!("sqlite:{}", self.test_db_path)
        }
    }
    
    /// Get human-readable description of database being used
    pub fn get_description(&self) -> String {
        if self.use_production_db {
            "Production Database (db/stocks.db)".to_string()
        } else {
            format!("Test Database ({})", self.test_db_path)
        }
    }
}