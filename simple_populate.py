#!/usr/bin/env python3
"""
Simple script to populate the SQLite database with S&P 500 stock symbols
This bypasses API issues and gives us data to test the UI with
"""

import sqlite3
from datetime import datetime, date
import random

# S&P 500 symbols (same as in our Rust code)
SP500_SYMBOLS = [
    # Technology
    "AAPL", "MSFT", "GOOGL", "GOOG", "AMZN", "NVDA", "META", "TSLA", "NFLX", "ADBE",
    "CRM", "ORCL", "CSCO", "INTC", "IBM", "QCOM", "TXN", "AVGO", "INTU", "AMD",
    "NOW", "AMAT", "ADI", "LRCX", "KLAC", "CDNS", "SNPS", "MCHP", "FTNT", "PANW",
    
    # Healthcare
    "UNH", "JNJ", "PFE", "ABBV", "LLY", "TMO", "ABT", "MRK", "DHR", "BMY",
    "AMGN", "MDT", "GILD", "CVS", "CI", "ISRG", "REGN", "VRTX", "ZTS", "DXCM",
    "BDX", "ELV", "HUM", "SYK", "BSX", "A", "EW", "IDXX", "RMD", "IQV",
    
    # Financial Services
    "BRK-B", "JPM", "V", "MA", "BAC", "WFC", "GS", "MS", "C", "AXP",
    "SPGI", "BLK", "SCHW", "CB", "ICE", "PGR", "AON", "CME", "USB", "TFC",
    "COF", "AIG", "MET", "PRU", "TRV", "ALL", "AJG", "MMC", "PNC", "BK",
    
    # Consumer Discretionary
    "HD", "MCD", "NKE", "SBUX", "LOW", "TJX", "BKNG", "GM", "F", 
    "ABNB", "MAR", "HLT", "MGM", "WYNN", "LVS", "NCLH", "RCL", "CCL",
    "YUM", "CMG", "DPZ", "QSR", "KMX", "BBY", "TGT", "WMT", "COST",
    
    # Consumer Staples  
    "PG", "KO", "PEP", "MDLZ", "CL", "GIS", "K", "CPB",
    "CAG", "SJM", "HSY", "MKC", "CLX", "KR", "SYY", "ADM", "TSN", "HRL",
    
    # Energy
    "XOM", "CVX", "COP", "EOG", "SLB", "PXD", "VLO", "MPC", "PSX", "KMI",
    "OKE", "WMB", "HES", "DVN", "FANG", "BKR", "HAL", "MRO", "APA", "OXY",
]

def populate_database():
    print("🔄 Populating database with S&P 500 stock data...")
    
    # Connect to database
    conn = sqlite3.connect('stocks.db')
    cursor = conn.cursor()
    
    # Add stocks
    stocks_added = 0
    today = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
    
    for symbol in SP500_SYMBOLS:
        try:
            cursor.execute("""
                INSERT OR IGNORE INTO stocks 
                (symbol, company_name, sector, industry, market_cap, status, first_trading_date, last_updated, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            """, (
                symbol,
                f"{symbol} Inc.",  # Placeholder company name
                "Technology" if symbol in ["AAPL", "MSFT", "GOOGL", "NVDA", "META"] else "Various",
                None,
                random.uniform(1000000000, 3000000000000),  # Random market cap between 1B-3T
                "active",
                "2020-01-01",
                today,
                today
            ))
            
            if cursor.rowcount > 0:
                stocks_added += 1
                
        except Exception as e:
            print(f"❌ Error adding {symbol}: {e}")
    
    # Add some sample price data for a few stocks to test P/E analysis
    sample_stocks = ["AAPL", "MSFT", "GOOGL", "AMZN", "NVDA"]
    prices_added = 0
    
    print(f"📊 Adding sample price data for {len(sample_stocks)} stocks...")
    
    for symbol in sample_stocks:
        # Get stock ID
        cursor.execute("SELECT id FROM stocks WHERE symbol = ?", (symbol,))
        result = cursor.fetchone()
        if result:
            stock_id = result[0]
            
            # Add some sample daily prices with P/E ratios
            base_price = random.uniform(100, 400)
            for i in range(10):  # Last 10 days
                date_str = f"2025-08-{20+i:02d}"
                price = base_price * (1 + random.uniform(-0.05, 0.05))
                pe_ratio = random.uniform(15, 35)
                
                cursor.execute("""
                    INSERT OR IGNORE INTO daily_prices
                    (stock_id, date, open_price, high_price, low_price, close_price, volume, pe_ratio, market_cap, dividend_yield)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """, (
                    stock_id,
                    date_str,
                    price,
                    price * 1.02,
                    price * 0.98,
                    price,
                    random.randint(1000000, 50000000),
                    pe_ratio,
                    random.uniform(1000000000000, 3000000000000),
                    random.uniform(1.0, 4.0)
                ))
                
                if cursor.rowcount > 0:
                    prices_added += 1
    
    # Update metadata
    cursor.execute("INSERT OR REPLACE INTO metadata (key, value, updated_at) VALUES (?, ?, ?)",
                   ("last_update_date", "2025-08-29", today))
    
    # Commit changes
    conn.commit()
    conn.close()
    
    print(f"✅ Database populated successfully!")
    print(f"   Added {stocks_added} stocks")
    print(f"   Added {prices_added} price records")
    print(f"   Total symbols: {len(SP500_SYMBOLS)}")

if __name__ == "__main__":
    populate_database()