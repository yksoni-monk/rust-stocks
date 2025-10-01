import { test, expect } from '@playwright/test';

test.describe('O\'Shaughnessy Button Test', () => {
  test('should click O\'Shaughnessy button and check for errors', async ({ page }) => {
    console.log('🚀 Starting O\'Shaughnessy button test...');

    await page.goto('/');

    // Wait for app title to load
    await expect(page.locator('h1')).toContainText('Stock Analysis Dashboard');
    console.log('✅ App loaded successfully');

    // Click on Stock Screening tab
    await page.click('text=🔍 Stock Screening');
    console.log('✅ Clicked on Stock Screening tab');

    // Wait for the screening interface to load
    await page.waitForTimeout(2000);

    // Take a screenshot to see what's on the page
    await page.screenshot({ path: 'before-oshaughnessy-click.png', fullPage: true });

    // Look for the O'Shaughnessy button
    const oshaughnessyButton = page.locator('button:has-text("O\'Shaughnessy")');
    await expect(oshaughnessyButton).toBeVisible({ timeout: 10000 });
    console.log('✅ Found O\'Shaughnessy button');

    // Click the O'Shaughnessy button
    await oshaughnessyButton.click();
    console.log('✅ Clicked O\'Shaughnessy button');

    // Wait for the loading to start and finish
    await page.waitForTimeout(1000);

    // Look for loading spinner
    const loadingSpinner = page.locator('text=Analyzing..., .animate-spin');
    if (await loadingSpinner.isVisible()) {
      console.log('⏳ Loading spinner visible, waiting for completion...');
      await page.waitForSelector('text=Analyzing...', { state: 'detached', timeout: 20000 });
      console.log('✅ Loading completed');
    }

    // Wait additional time for results to render
    await page.waitForTimeout(3000);

    // Take a screenshot after clicking
    await page.screenshot({ path: 'after-oshaughnessy-click.png', fullPage: true });

    // Check for error messages specifically
    const errorMessages = [
      'Analysis Error',
      'Failed to load recommendations',
      'Database query failed',
      'Error:'
    ];

    let foundError = false;
    let errorText = '';

    for (const errorMsg of errorMessages) {
      const errorElement = page.locator(`text=${errorMsg}`);
      if (await errorElement.isVisible()) {
        foundError = true;
        errorText = await errorElement.textContent() || '';
        console.log(`❌ Found error: "${errorText}"`);
        break;
      }
    }

    if (foundError) {
      // Take error screenshot
      await page.screenshot({ path: 'oshaughnessy-error.png', fullPage: true });

      // Log all visible text to help debug
      const pageContent = await page.locator('body').textContent();
      console.log('📄 Full page content:', pageContent?.substring(0, 1000) + '...');

      // This test should fail to highlight the error
      expect(foundError).toBeFalsy(`Found error: ${errorText}`);
    } else {
      console.log('✅ No error messages found');

      // Look for results/recommendations panel
      const resultsPanel = page.locator('[data-section="recommendations"]');
      const resultsVisible = await resultsPanel.isVisible();

      if (resultsVisible) {
        console.log('✅ Results panel is visible');

        // Look for table or results content
        const table = page.locator('table');
        const hasTable = await table.isVisible();

        if (hasTable) {
          const rowCount = await page.locator('tbody tr').count();
          console.log(`📊 Found ${rowCount} result rows`);

          if (rowCount > 0) {
            console.log('✅ O\'Shaughnessy screening appears to be working!');
          } else {
            console.log('⚠️ Results table is empty');
          }
        } else {
          console.log('⚠️ No table found in results panel');
        }
      } else {
        console.log('⚠️ Results panel is not visible');
      }
    }

    console.log('🏁 Test completed');
  });
});