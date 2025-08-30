#!/usr/bin/env python3
"""
Schwab API Token Management Script

This script helps manage Schwab API tokens by:
1. Checking token expiration status
2. Refreshing expired tokens using the refresh token
3. Displaying token information

Usage:
    python refresh_token.py [--check|--refresh]
"""

import json
import requests
import base64
import time
from datetime import datetime
from pathlib import Path

# Configuration
TOKEN_FILE = "schwab_tokens.json"
ENV_FILE = ".env"

def load_config():
    """Load API credentials from .env file"""
    config = {}
    if Path(ENV_FILE).exists():
        with open(ENV_FILE, 'r') as f:
            for line in f:
                if '=' in line and not line.strip().startswith('#'):
                    key, value = line.strip().split('=', 1)
                    config[key] = value
    return config

def load_tokens():
    """Load tokens from JSON file"""
    if not Path(TOKEN_FILE).exists():
        print(f"❌ Token file {TOKEN_FILE} not found")
        return None
    
    with open(TOKEN_FILE, 'r') as f:
        return json.load(f)

def save_tokens(token_data):
    """Save tokens to JSON file"""
    with open(TOKEN_FILE, 'w') as f:
        json.dump(token_data, f, indent=2)
    print(f"✅ Tokens saved to {TOKEN_FILE}")

def check_token_status():
    """Check and display token status"""
    data = load_tokens()
    if not data:
        return False
    
    expires_at = data['token']['expires_at']
    creation = data['creation_timestamp']
    now = time.time()
    
    print("📊 Token Status:")
    print(f"   Creation: {datetime.fromtimestamp(creation)}")
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

def refresh_access_token():
    """Refresh the access token using the refresh token"""
    print("🔄 Refreshing access token...")
    
    config = load_config()
    token_data = load_tokens()
    
    if not config.get('SCHWAB_API_KEY') or not config.get('SCHWAB_APP_SECRET'):
        print("❌ Missing SCHWAB_API_KEY or SCHWAB_APP_SECRET in .env file")
        return False
    
    if not token_data:
        return False
    
    # Prepare refresh request
    api_key = config['SCHWAB_API_KEY']
    app_secret = config['SCHWAB_APP_SECRET']
    refresh_token = token_data['token']['refresh_token']
    
    # Create authorization header
    credentials = f"{api_key}:{app_secret}"
    encoded_credentials = base64.b64encode(credentials.encode()).decode()
    
    headers = {
        'Authorization': f'Basic {encoded_credentials}',
        'Content-Type': 'application/x-www-form-urlencoded'
    }
    
    data = {
        'grant_type': 'refresh_token',
        'refresh_token': refresh_token
    }
    
    try:
        response = requests.post(
            'https://api.schwabapi.com/v1/oauth/token',
            headers=headers,
            data=data
        )
        
        if response.status_code == 200:
            new_token = response.json()
            
            # Update token data
            updated_data = {
                'creation_timestamp': time.time(),
                'token': {
                    'expires_in': new_token['expires_in'],
                    'token_type': new_token['token_type'],
                    'scope': new_token['scope'],
                    'refresh_token': new_token['refresh_token'],
                    'access_token': new_token['access_token'],
                    'id_token': new_token.get('id_token', ''),
                    'expires_at': time.time() + new_token['expires_in']
                }
            }
            
            save_tokens(updated_data)
            print("✅ Token refreshed successfully!")
            return True
            
        else:
            print(f"❌ Token refresh failed: {response.status_code}")
            print(f"   Response: {response.text}")
            return False
            
    except Exception as e:
        print(f"❌ Error refreshing token: {e}")
        return False

def main():
    import sys
    
    if len(sys.argv) > 1:
        command = sys.argv[1]
        if command == '--check':
            check_token_status()
        elif command == '--refresh':
            refresh_access_token()
        else:
            print("Usage: python refresh_token.py [--check|--refresh]")
    else:
        # Default: check status and refresh if needed
        print("🔍 Checking token status...")
        if not check_token_status():
            print("\n🔄 Token expired, attempting to refresh...")
            if refresh_access_token():
                print("\n✅ Token refreshed. Checking status again...")
                check_token_status()
            else:
                print("❌ Failed to refresh token")

if __name__ == "__main__":
    main()