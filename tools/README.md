# Tools Directory

This directory contains utility tools for the rust-stocks project.

## Available Tools

### `update_sp500.rs`
Updates the S&P 500 company list with state tracking.

**Usage:**
```bash
cargo run --bin update_sp500
```

**Features:**
- Fetches current S&P 500 companies from Schwab API
- Updates local database with company information
- Tracks update state to avoid unnecessary API calls
- Provides detailed logging of the update process

## Building Tools

All tools can be built and run using:
```bash
cargo run --bin <tool_name>
```

Or built individually:
```bash
cargo build --bin <tool_name>
```
