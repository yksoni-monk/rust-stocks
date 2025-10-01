import { test, expect } from '@playwright/test';

test.describe('O\'Shaughnessy Value Composite Screening', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the app
    await page.goto('/');

    // Wait for the app to load
    await page.waitForSelector('[data-testid="app-container"]', { timeout: 10000 });
  });

  test('should successfully load O\'Shaughnessy screening results', async ({ page }) => {
    // Navigate to the screening section
    await page.click('[data-testid="screening-tab"]', { timeout: 5000 });

    // Wait for screening interface to load
    await expect(page.locator('[data-testid="screening-interface"]')).toBeVisible();

    // Select O'Shaughnessy screening method
    await page.selectOption('[data-testid="screening-type-select"]', 'oshaughnessy');

    // Wait a moment for the selection to register
    await page.waitForTimeout(1000);

    // Click the "Run Analysis" or similar button
    await page.click('[data-testid="run-analysis-button"]', { timeout: 5000 });

    // Wait for loading to complete and results to appear
    await page.waitForSelector('[data-testid="recommendations-table"]', { timeout: 15000 });

    // Verify that we have recommendation results
    const recommendationRows = page.locator('[data-testid="recommendation-row"]');
    await expect(recommendationRows).toHaveCountGreaterThan(0);

    // Verify that O'Shaughnessy-specific data is displayed
    const firstRow = recommendationRows.first();

    // Check for O'Shaughnessy metrics in the first row
    await expect(firstRow.locator('[data-testid="ps-ratio"]')).toBeVisible();
    await expect(firstRow.locator('[data-testid="pb-ratio"]')).toBeVisible();
    await expect(firstRow.locator('[data-testid="composite-score"]')).toBeVisible();

    // Verify no error messages are shown
    await expect(page.locator('[data-testid="error-message"]')).not.toBeVisible();
    await expect(page.locator('text=Analysis Error')).not.toBeVisible();
    await expect(page.locator('text=Failed to load recommendations')).not.toBeVisible();
  });

  test('should display all 6 O\'Shaughnessy metrics', async ({ page }) => {
    // Navigate to screening and select O'Shaughnessy
    await page.click('[data-testid="screening-tab"]');
    await page.selectOption('[data-testid="screening-type-select"]', 'oshaughnessy');
    await page.click('[data-testid="run-analysis-button"]');

    // Wait for results
    await page.waitForSelector('[data-testid="recommendations-table"]', { timeout: 15000 });

    // Check that all 6 metrics are displayed in the results
    const metricsContainer = page.locator('[data-testid="metrics-display"]');

    await expect(metricsContainer.locator('text=P/S Ratio')).toBeVisible();
    await expect(metricsContainer.locator('text=EV/S Ratio')).toBeVisible();
    await expect(metricsContainer.locator('text=P/E Ratio')).toBeVisible();
    await expect(metricsContainer.locator('text=P/B Ratio')).toBeVisible();
    await expect(metricsContainer.locator('text=EV/EBITDA')).toBeVisible();
    await expect(metricsContainer.locator('text=Shareholder Yield')).toBeVisible();
  });

  test('should handle O\'Shaughnessy criteria filtering', async ({ page }) => {
    // Navigate to screening and select O'Shaughnessy
    await page.click('[data-testid="screening-tab"]');
    await page.selectOption('[data-testid="screening-type-select"]', 'oshaughnessy');

    // Adjust criteria
    await page.fill('[data-testid="max-composite-percentile"]', '10');
    await page.fill('[data-testid="max-ps-ratio"]', '1.5');

    // Run analysis
    await page.click('[data-testid="run-analysis-button"]');

    // Wait for results
    await page.waitForSelector('[data-testid="recommendations-table"]', { timeout: 15000 });

    // Verify results are filtered (should have fewer results with stricter criteria)
    const recommendationRows = page.locator('[data-testid="recommendation-row"]');
    const rowCount = await recommendationRows.count();

    // Should have some results but likely fewer than default
    expect(rowCount).toBeGreaterThan(0);
    expect(rowCount).toBeLessThanOrEqual(20);
  });

  test('should show proper error handling for invalid criteria', async ({ page }) => {
    // Navigate to screening and select O'Shaughnessy
    await page.click('[data-testid="screening-tab"]');
    await page.selectOption('[data-testid="screening-type-select"]', 'oshaughnessy');

    // Set invalid criteria that would return no results
    await page.fill('[data-testid="max-composite-percentile"]', '0.1');
    await page.fill('[data-testid="min-market-cap"]', '999999999999');

    // Run analysis
    await page.click('[data-testid="run-analysis-button"]');

    // Should either show no results message or handle gracefully
    await page.waitForTimeout(5000);

    const errorMessage = page.locator('[data-testid="no-results-message"]');
    const emptyTable = page.locator('[data-testid="empty-recommendations"]');

    // Either should show a "no results" message or empty state, not an error
    const hasNoResults = await errorMessage.isVisible() || await emptyTable.isVisible();
    expect(hasNoResults).toBeTruthy();

    // Should NOT show analysis error
    await expect(page.locator('text=Analysis Error')).not.toBeVisible();
  });
});