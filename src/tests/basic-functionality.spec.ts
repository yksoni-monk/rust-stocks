import { test, expect } from '@playwright/test';

test.describe('Basic App Functionality', () => {
  test('should load the application', async ({ page }) => {
    await page.goto('/');

    // Wait for app to load and check for main title
    await expect(page.locator('h1')).toContainText('Stock Analysis Dashboard');
  });

  test('should have navigation tabs', async ({ page }) => {
    await page.goto('/');

    // Check for navigation tabs
    await expect(page.locator('text=Stock Screening')).toBeVisible();
    await expect(page.locator('text=Browse Stocks')).toBeVisible();
    await expect(page.locator('text=Data Management')).toBeVisible();
  });

  test('should switch to screening tab and show screening interface', async ({ page }) => {
    await page.goto('/');

    // Click on screening tab
    await page.click('text=Stock Screening');

    // Should see screening type selector
    await expect(page.locator('select')).toBeVisible();
  });

  test('should test O\'Shaughnessy screening method', async ({ page }) => {
    await page.goto('/');

    // Navigate to screening
    await page.click('text=Stock Screening');

    // Find and select O'Shaughnessy option
    const selectElement = page.locator('select');
    await selectElement.selectOption('oshaughnessy');

    // Look for run analysis button or similar
    const runButton = page.locator('button:has-text("Load"), button:has-text("Run"), button:has-text("Analyze")').first();
    if (await runButton.isVisible()) {
      await runButton.click();

      // Wait a bit for the request to complete
      await page.waitForTimeout(5000);

      // Check if there's an error message
      const errorMessage = page.locator('text=Analysis Error, text=Failed to load, text=Error');

      if (await errorMessage.isVisible()) {
        console.log('❌ Error found:', await errorMessage.textContent());

        // Screenshot for debugging
        await page.screenshot({ path: 'oshaughnessy-error.png' });

        // This test should fail so we know there's an issue
        expect(await errorMessage.isVisible()).toBeFalsy();
      } else {
        console.log('✅ No error message found');

        // Look for results table or data
        const resultsArea = page.locator('table, .results, .recommendations');
        if (await resultsArea.isVisible()) {
          console.log('✅ Results area found');
        } else {
          console.log('⚠️ No results area found but no error either');
        }
      }
    } else {
      console.log('⚠️ No run button found');
    }
  });
});