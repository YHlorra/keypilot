import { test, expect } from '@playwright/test';

test('UsagePage layout + data', async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      const text = msg.text();
      if (!text.includes('tauri') && !text.includes('WebView')) {
        consoleErrors.push(text);
      }
    }
  });

  await page.addInitScript(() => {
    const sampleDaily = [];
    const today = new Date();
    for (let i = 60; i >= 0; i--) {
      const d = new Date(today);
      d.setDate(d.getDate() - i);
      const iso = d.toISOString().split('T')[0];
      const requests = Math.floor(Math.random() * 5000) + (i < 7 ? 1000 : 0);
      sampleDaily.push({
        date: iso,
        total_tokens: requests * 1200,
        total_cost_usd: requests * 0.012,
        request_count: requests,
      });
    }
    const sampleSummary = {
      total_tokens: sampleDaily.reduce((a: number, b: any) => a + b.total_tokens, 0),
      total_cost_usd: sampleDaily.reduce((a: number, b: any) => a + b.total_cost_usd, 0),
      total_requests: sampleDaily.reduce((a: number, b: any) => a + b.request_count, 0),
      agent_pairs: [
        { agent_type: 'opencode', model: 'MiniMax-M2.7', provider: 'minimax-cn-coding-plan', total_tokens: 4500000, total_cost_usd: 45.0, request_count: 1200, token_breakdown: { input: 3000000, output: 1500000 } },
        { agent_type: 'claude-code', model: 'claude-opus-4-7', provider: 'anthropic', total_tokens: 2300000, total_cost_usd: 89.5, request_count: 480, token_breakdown: { input: 1200000, output: 1100000 } },
      ],
      daily_series: sampleDaily,
    };
    (window as any).__TAURI_INTERNALS__ = {
      invoke: async (cmd: string, args?: any) => {
        if (cmd === 'plugin:event|listen' || cmd === 'plugin:event|unlisten') return 0;
        if (cmd === 'get_usage_summary') {
          const f = args?.filter ?? {};
          let series = sampleSummary.daily_series;
          if (f.start_date) series = series.filter((d: any) => d.date >= f.start_date);
          if (f.end_date) series = series.filter((d: any) => d.date <= f.end_date);
          return { ...sampleSummary, daily_series: series };
        }
        if (cmd === 'list_providers' || cmd === 'get_provider' || cmd === 'list_categories') return [];
        return null;
      },
    };
  });

  await page.goto('/');
  await page.getByRole('button', { name: /usage/i }).click();
  await page.waitForTimeout(2000);

  // Header elements visible
  const h1 = page.locator('h1').filter({ hasText: 'Usage' });
  await expect(h1).toBeVisible();

  // KPI cards: look for Today, Last 7 days, Last 30 days labels
  const kpiLabels = page.getByText(/Today|Last 7 days|Last 30 days/i);
  expect(await kpiLabels.count()).toBeGreaterThanOrEqual(3);

  // Trend chart SVG visible
  const svgVisible = page.locator('svg').first().isVisible();
  expect(svgVisible).toBeTruthy();

  // Heatmap calendar: 182 cells (7 rows × 26 cols)
  const heatmapCells = page.locator('div.rounded-sm.cursor-pointer');
  const cellCount = await heatmapCells.count();
  expect(cellCount).toBeGreaterThanOrEqual(170);
  expect(cellCount).toBeLessThanOrEqual(195);

  // Today KPI shows number > 0
  const todayCard = page.locator('text=Today').first().locator('..');
  const todayCardText = await todayCard.textContent();
  const todayMatch = todayCardText?.match(/[\d,]+/);
  expect(todayMatch).toBeTruthy();
  const todayNum = parseInt(todayMatch![0].replace(/,/g, ''), 10);
  expect(todayNum).toBeGreaterThan(0);

  // No console errors
  expect(consoleErrors).toHaveLength(0);

  // Screenshot
  await page.screenshot({ path: 'tests/__screenshots__/usage-page.png', fullPage: true });
});
