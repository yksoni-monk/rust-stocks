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
SCHWAB_TOKEN_FILE = "schwab_tokens_full.json"  # schwab-py's full token file
MINIMAL_TOKEN_FILE = "schwab_tokens.json"  # Your minimal file
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

def save_minimal_tokens(full_token_file):
    """Load full token from schwab-py file and save minimal structure"""
    try:
        if not Path(full_token_file).exists():
            print(f"❌ Full token file {full_token_file} not found")
            return False
        with open(full_token_file, 'r') as f:
            data = json.load(f)
        token = data.get('token', {})
        required_keys = ['access_token', 'refresh_token', 'expires_in']
        if not all(k in token for k in required_keys):
            print(f"❌ Invalid token data: missing required keys")
            print(f"   Token: {token}")
            return False
        
        updated_data = {
            'access_token': token['access_token'],
            'refresh_token': token['refresh_token'],
            'expires_at': time.time() + token['expires_in']
        }
        with open(MINIMAL_TOKEN_FILE, 'w') as f:
            json.dump(updated_data, f, indent=2)
        print(f"✅ Minimal tokens saved to {MINIMAL_TOKEN_FILE}")
        return True
    except Exception as e:
        print(f"❌ Error saving minimal tokens: {e}")
        return False

def check_token_status(token_file):
    """Check token status from minimal file"""
    if not Path(token_file).exists():
        print(f"❌ Token file {token_file} not found")
        return False
    with open(token_file, 'r') as f:
        data = json.load(f)
    expires_at = data['expires_at']
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
        if save_minimal_tokens(SCHWAB_TOKEN_FILE):
            print("✅ Initial authentication complete!")
        else:
            print("❌ Failed to save tokens")
    elif len(sys.argv) > 1 and sys.argv[1] == '--refresh':
        # Load and refresh if needed
        print("🔍 Checking token status...")
        if check_token_status(MINIMAL_TOKEN_FILE):
            print("✅ Access token is valid, no refresh needed.")
        else:
            print("\n🔄 Loading client (auto-refreshes if refresh token valid)...")
            try:
                client = client_from_token_file(SCHWAB_TOKEN_FILE, api_key, app_secret)
                # Test with a simple API call
                response = client.get_account_numbers()
                if response.status_code == 200:
                    print("✅ Refresh successful! API call works.")
                    # Save updated minimal tokens
                    save_minimal_tokens(SCHWAB_TOKEN_FILE)
                else:
                    print(f"❌ Refresh failed: {response.status_code}, {response.text}")
                    print("ℹ️ Run with --auth to re-authenticate.")
            except Exception as e:
                print(f"❌ Error loading client: {e}")
                print("ℹ️ Run with --auth to re-authenticate.")
    else:
        print("Usage: python schwab_auth.py [--auth | --refresh]")
        sys.exit(1)

if __name__ == "__main__":
    main()