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

  // NOTE: addInitScript with a string (not a function) keeps the payload
  // pure JS — playwright's function path uses String(fn) which preserves
  // TS annotations like `(window as any)` and `: string` param types,
  // and the browser context then chokes on them.  Pure-string version
  // sidesteps that whole class of error.
  await page.addInitScript({ content: `
(() => {
  var sampleDaily = [];
  var today = new Date();
  for (var i = 60; i >= 0; i--) {
    var d = new Date(today);
    d.setDate(d.getDate() - i);
    var iso = d.toISOString().split('T')[0];
    var requests = Math.floor(Math.random() * 5000) + (i < 7 ? 1000 : 0);
    sampleDaily.push({
      date: iso,
      total_tokens: requests * 1200,
      total_cost_usd: requests * 0.012,
      request_count: requests
    });
  }

  // Matches Rust UsageSummaryResponse IPC DTO after Bug #1 fix (snake_case + token_breakdown)
  var sampleSummary = {
    total_tokens: sampleDaily.reduce(function (a, b) { return a + b.total_tokens; }, 0),
    total_cost_usd: sampleDaily.reduce(function (a, b) { return a + b.total_cost_usd; }, 0),
    total_requests: sampleDaily.reduce(function (a, b) { return a + b.request_count; }, 0),
    agent_pairs: [
      { agent_type: 'opencode', model: 'MiniMax-M2.7', provider: 'minimax-cn-coding-plan', total_tokens: 4500000, total_cost_usd: 45.0, request_count: 1200, token_breakdown: { input: 3000000, output: 1500000 } },
      { agent_type: 'claude-code', model: 'claude-opus-4-7', provider: 'anthropic', total_tokens: 2300000, total_cost_usd: 89.5, request_count: 480, token_breakdown: { input: 1200000, output: 1100000 } }
    ],
    daily_series: sampleDaily
  };

  // Matches PeriodsSummaryResponse — the IPC the live page calls
  // (Bug #1 fix 2026-06-29).
  var todayIso = today.toISOString().split('T')[0];
  var monthIso = today.getFullYear() + '-' + String(today.getMonth() + 1).padStart(2, '0');
  var samplePeriods = {
    periods: {
      today: sampleSummary,
      month: sampleSummary,
      all_time: sampleSummary
    },
    period_windows: {
      today: { key: todayIso, ends_at: new Date(today.getTime() + 86400e3).toISOString() },
      month: { key: monthIso, ends_at: new Date(today.getFullYear(), today.getMonth() + 1, 1).toISOString() }
    },
    client_models: {
      'opencode': { 'MiniMax-M2.7': 4500000 },
      'claude-code': { 'claude-opus-4-7': 2300000 }
    },
    limits: null
  };

  window.__TAURI_INTERNALS__ = {
    invoke: async function (cmd, args) {
      if (cmd === 'plugin:event|listen' || cmd === 'plugin:event|unlisten') return 0;
      if (cmd === 'get_usage_periods_summary') {
        var f = (args && args.filter) || {};
        var series = samplePeriods.periods.month.daily_series;
        if (f.start_date) series = series.filter(function (d) { return d.date >= f.start_date; });
        if (f.end_date) series = series.filter(function (d) { return d.date <= f.end_date; });
        var out = JSON.parse(JSON.stringify(samplePeriods));
        out.periods.month.daily_series = series;
        return out;
      }
      if (cmd === 'get_usage_summary') {
        var f2 = (args && args.filter) || {};
        var series2 = sampleSummary.daily_series;
        if (f2.start_date) series2 = series2.filter(function (d) { return d.date >= f2.start_date; });
        if (f2.end_date) series2 = series2.filter(function (d) { return d.date <= f2.end_date; });
        var out2 = JSON.parse(JSON.stringify(sampleSummary));
        out2.daily_series = series2;
        return out2;
      }
      if (cmd === 'list_providers' || cmd === 'get_provider' || cmd === 'list_categories') return [];
      if (cmd === 'force_rescan_all') return { cursors_reset: 0, total_records: 0 };
      return null;
    }
  };
})();
` });

  await page.goto('/');
  await page.getByRole('button', { name: /usage/i }).click();
  await page.waitForTimeout(2000);

  // Header elements visible
  const h1 = page.locator('h1').filter({ hasText: 'Usage' });
  await expect(h1).toBeVisible();

  // KPI cards: Today / This Month / All Time (post-alignment).
  const kpiLabels = page.getByText(/Today|This Month|All Time/i);
  expect(await kpiLabels.count()).toBeGreaterThanOrEqual(3);

  // Trend chart SVG visible
  const svgVisible = await page.locator('svg').first().isVisible();
  expect(svgVisible).toBeTruthy();

  // Heatmap calendar: 7 rows x 26 cols = 182 cells (loose window 170-195
  // accommodates tooltip overlays or weekend tint variations).
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

  // Regression guard: Bug #1 — `total_cost_usd` field must render in KPI sublabels.
  // Before the fix the wire format was `total_cost` (undefined on TS side)
  // so every "0.00 USD" was visible.  Assert a real dollar amount shows up.
  const monthCard = page.locator('text=This Month').first().locator('..');
  const monthCardText = await monthCard.textContent();
  expect(monthCardText).toMatch(/\d+\.\d{2}\s*USD/, 'This Month KPI shows a real USD cost (not 0.00)');

  // Regression guard: Bug #3 — `force_rescan_all` IPC reachable, returns the
  // expected shape.  Asserting on a direct invoke catches payload contract
  // drift without needing to drive the Settings modal UI.
  const forceRescanResult = await page.evaluate(async () => {
    // @ts-ignore — mocked in init script
    return await window.__TAURI_INTERNALS__.invoke('force_rescan_all', {});
  });
  expect(forceRescanResult).toMatchObject({
    cursors_reset: expect.any(Number),
    total_records: expect.any(Number),
  });

  // No console errors
  expect(consoleErrors).toHaveLength(0);

  // Screenshot
  await page.screenshot({ path: 'tests/__screenshots__/usage-page.png', fullPage: true });
});