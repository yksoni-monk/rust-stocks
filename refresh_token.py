#!/usr/bin/env python3
"""
Schwab API Authentication with schwab-py

Run for initial auth: python schwab_auth.py --auth
Run for refresh/check: python schwab_auth.py --refresh
"""

import sys
from pathlib import Path
from schwab.auth import client_from_manual_flow, client_from_token_file
from schwab.client import Client
import json
import time
from datetime import datetime

# Configuration
SCHWAB_TOKEN_FILE = "src-tauri/schwab_tokens.json"  # Single token file
ENV_FILE = ".env"
CALLBACK_URL = "https://127.0.0.1:8182"

def load_config():
    """Load API credentials from .env file"""
    config = {}
    if Path(ENV_FILE).exists():
        with open(ENV_FILE, 'r') as f:
            for line in f:
                if '=' in line and not line.strip().startswith('#'):
                    key, value = line.strip().split('=', 1)
                    config[key] = value
    else:
        print(f"❌ {ENV_FILE} not found")
    return config

def save_tokens(token_data):
    """Save token data to the single token file"""
    try:
        with open(SCHWAB_TOKEN_FILE, 'w') as f:
            json.dump(token_data, f, indent=2)
        print(f"✅ Tokens saved to {SCHWAB_TOKEN_FILE}")
        return True
    except Exception as e:
        print(f"❌ Error saving tokens: {e}")
        return False

def check_token_status(token_file):
    """Check token status from token file"""
    if not Path(token_file).exists():
        print(f"❌ Token file {token_file} not found")
        return False
    with open(token_file, 'r') as f:
        data = json.load(f)
    
    # Handle both old and new token formats
    if 'expires_at' in data:
        # Old format: expires_at at root level
        expires_at = data['expires_at']
    elif 'token' in data and 'expires_at' in data['token']:
        # New format: expires_at in token object
        expires_at = data['token']['expires_at']
    else:
        print("❌ No expires_at found in token file")
        return False

    # Convert expires_at to timestamp
    if isinstance(expires_at, str):
        # Handle ISO 8601 datetime strings (e.g., "2025-10-08T00:35:03.531020Z")
        try:
            expires_dt = datetime.fromisoformat(expires_at.replace('Z', '+00:00'))
            expires_at = expires_dt.timestamp()
        except ValueError:
            # Try parsing as integer string
            try:
                expires_at = int(expires_at)
            except ValueError:
                print(f"❌ Invalid expires_at format: {expires_at}")
                return False

    now = time.time()
    print("📊 Token Status:")
    print(f"   Expires:  {datetime.fromtimestamp(expires_at)}")
    print(f"   Current:  {datetime.fromtimestamp(now)}")
    is_expired = expires_at < now
    time_left = expires_at - now
    if is_expired:
        print(f"❌ Token expired {abs(time_left):.0f} seconds ago")
    else:
        hours_left = time_left / 3600
        print(f"✅ Token valid for {hours_left:.1f} more hours")
    return not is_expired

def main():
    config = load_config()
    api_key = config.get('SCHWAB_API_KEY')
    app_secret = config.get('SCHWAB_APP_SECRET')

    if not api_key or not app_secret:
        print("❌ Missing SCHWAB_API_KEY or SCHWAB_APP_SECRET in .env file")
        sys.exit(1)

    if len(sys.argv) > 1 and sys.argv[1] == '--auth':
        # Initial authentication
        print("🔐 Starting initial authentication...")
        print("\n" + "="*60)
        print("Follow the schwab-py instructions in the terminal.")
        print("Paste the redirect URL IMMEDIATELY after clicking 'Allow' to avoid expiration.")
        print("="*60 + "\n")
        client = client_from_manual_flow(
            api_key, app_secret, CALLBACK_URL, token_path=SCHWAB_TOKEN_FILE
        )
        print("✅ Initial authentication complete!")
    elif len(sys.argv) > 1 and sys.argv[1] == '--refresh':
        # Load and refresh if needed
        print("🔍 Checking token status...")
        if check_token_status(SCHWAB_TOKEN_FILE):
            print("✅ Access token is valid, no refresh needed.")
        else:
            print("\n🔄 Loading client (auto-refreshes if refresh token valid)...")
            try:
                client = client_from_token_file(SCHWAB_TOKEN_FILE, api_key, app_secret)
                # Test with a simple API call
                response = client.get_account_numbers()
                if response.status_code == 200:
                    print("✅ Refresh successful! API call works.")
                else:
                    print(f"❌ Refresh failed: {response.status_code}, {response.text}")
                    print("ℹ️ Run with --auth to re-authenticate.")
            except Exception as e:
                print(f"❌ Error loading client: {e}")
                print("ℹ️ Run with --auth to re-authenticate.")
    else:
        print("Usage: python refresh_token.py [--auth | --refresh]")
        sys.exit(1)

if __name__ == "__main__":
    main()