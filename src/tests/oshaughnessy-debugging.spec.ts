import { test, expect } from '@playwright/test';

test.describe('O\'Shaughnessy Debugging', () => {
  test('debug O\'Shaughnessy screening functionality', async ({ page }) => {
    console.log('🚀 Starting O\'Shaughnessy debugging test...');

    await page.goto('/');

    // Wait for app title to load
    await expect(page.locator('h1')).toContainText('Stock Analysis Dashboard');
    console.log('✅ App loaded successfully');

    // Click on Stock Screening tab
    await page.click('text=🔍 Stock Screening');
    console.log('✅ Clicked on Stock Screening tab');

    // Wait a moment for the UI to settle
    await page.waitForTimeout(1000);

    // Take a screenshot to see what's on the page
    await page.screenshot({ path: 'screening-tab.png', fullPage: true });

    // Look for any select elements
    const selects = await page.locator('select').count();
    console.log(`📊 Found ${selects} select elements`);

    if (selects > 0) {
      const selectElement = page.locator('select').first();

      // Get all options
      const options = await selectElement.locator('option').allTextContents();
      console.log('📋 Available options:', options);

      // Try to select O'Shaughnessy if available
      if (options.some(opt => opt.toLowerCase().includes('oshaughnessy'))) {
        await selectElement.selectOption({ label: /oshaughnessy/i });
        console.log('✅ Selected O\'Shaughnessy option');

        // Take another screenshot after selection
        await page.screenshot({ path: 'after-oshaughnessy-selection.png', fullPage: true });

        // Wait a moment for any dynamic content to load
        await page.waitForTimeout(2000);

        // Look for any buttons that might trigger the analysis
        const buttons = await page.locator('button').all();
        console.log(`🔘 Found ${buttons.length} buttons`);

        for (let i = 0; i < buttons.length; i++) {
          const buttonText = await buttons[i].textContent();
          console.log(`Button ${i}: "${buttonText}"`);
        }

        // Look for specific analysis triggers
        const loadButton = page.locator('button:has-text("Load"), button:has-text("Run"), button:has-text("Analyze"), button:has-text("Get")').first();
        if (await loadButton.isVisible()) {
          console.log('🎯 Found analysis trigger button');
          await loadButton.click();
          console.log('✅ Clicked analysis button');

          // Wait for response
          await page.waitForTimeout(5000);

          // Take screenshot after clicking
          await page.screenshot({ path: 'after-analysis-click.png', fullPage: true });

          // Check for any error messages
          const errorElements = await page.locator('text=Error, text=Failed, text=Analysis Error').all();
          if (errorElements.length > 0) {
            console.log('❌ Found error messages:');
            for (const error of errorElements) {
              const errorText = await error.textContent();
              console.log(`  - ${errorText}`);
            }
          } else {
            console.log('✅ No error messages found');
          }

          // Look for any results
          const tables = await page.locator('table').count();
          const resultDivs = await page.locator('[class*="result"], [class*="recommendation"]').count();
          console.log(`📊 Found ${tables} tables and ${resultDivs} result containers`);

        } else {
          console.log('⚠️  No analysis trigger button found');
        }

      } else {
        console.log('❌ O\'Shaughnessy option not found in select');
      }
    } else {
      console.log('❌ No select elements found on the page');
    }

    // Final screenshot
    await page.screenshot({ path: 'final-state.png', fullPage: true });
    console.log('📸 Screenshots saved for debugging');
  });
});