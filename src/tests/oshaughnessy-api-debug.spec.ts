import { test, expect } from '@playwright/test';

test.describe('O\'Shaughnessy API Debug', () => {
  test('should capture O\'Shaughnessy API response and debug the issue', async ({ page }) => {
    console.log('🔍 Starting O\'Shaughnessy API debug test...');

    // Listen for console messages from the browser
    page.on('console', (msg) => {
      const text = msg.text();
      if (text.includes('O\'Shaughnessy') || text.includes('Loading') || text.includes('Error') || text.includes('result')) {
        console.log(`🌐 Browser Console: ${text}`);
      }
    });

    // Listen for API requests
    page.on('request', (request) => {
      const url = request.url();
      if (url.includes('oshaughnessy')) {
        console.log(`📡 API Request: ${request.method()} ${url}`);
        console.log(`📡 Request payload:`, request.postData());
      }
    });

    // Listen for API responses
    page.on('response', async (response) => {
      const url = response.url();
      if (url.includes('oshaughnessy')) {
        console.log(`📨 API Response: ${response.status()} ${url}`);
        try {
          const responseBody = await response.text();
          console.log(`📨 Response body:`, responseBody.substring(0, 500) + '...');
        } catch (e) {
          console.log(`📨 Could not read response body: ${e}`);
        }
      }
    });

    await page.goto('/');

    // Wait for app title to load
    await expect(page.locator('h1')).toContainText('Stock Analysis Dashboard');
    console.log('✅ App loaded successfully');

    // Click on Stock Screening tab
    await page.click('text=🔍 Stock Screening');
    console.log('✅ Clicked on Stock Screening tab');

    // Wait for the screening interface to load
    await page.waitForTimeout(2000);

    // Click the O'Shaughnessy button
    const oshaughnessyButton = page.locator('button:has-text("O\'Shaughnessy")');
    await expect(oshaughnessyButton).toBeVisible({ timeout: 10000 });

    console.log('🎯 Clicking O\'Shaughnessy button...');
    await oshaughnessyButton.click();
    console.log('✅ Clicked O\'Shaughnessy button');

    // Wait longer for all API calls and data processing
    console.log('⏳ Waiting for API calls to complete...');
    await page.waitForTimeout(8000);

    // Check final state
    const hasResults = await page.locator('[data-section="recommendations"]').isVisible();
    console.log(`📊 Results panel visible: ${hasResults}`);

    if (hasResults) {
      // Check for loading state
      const isLoading = await page.locator('text=Analyzing stocks').isVisible();
      console.log(`⏳ Still loading: ${isLoading}`);

      // Check for error state
      const hasError = await page.locator('text=Analysis Error').isVisible();
      console.log(`❌ Has error: ${hasError}`);

      if (hasError) {
        const errorText = await page.locator('text=Analysis Error').first().textContent();
        console.log(`❌ Error text: ${errorText}`);
      }

      // Check for "No Stocks Found"
      const hasNoResults = await page.locator('text=No Stocks Found').isVisible();
      console.log(`📭 No stocks found: ${hasNoResults}`);

      // Check for actual results
      const resultCards = await page.locator('[class*="bg-white rounded-lg"]').count();
      console.log(`📋 Result cards found: ${resultCards}`);

      // Final screenshot
      await page.screenshot({ path: 'oshaughnessy-final-debug.png', fullPage: true });
    }

    console.log('🏁 API debug test completed');
  });
});