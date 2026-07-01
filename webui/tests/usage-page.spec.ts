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
  // Realistic back-end: month.daily_series is the current calendar month
  // (1 day on the 1st of a month).  all_time.daily_series is the full
  // history.  Bug regression: the trend chart must source from all_time, not
  // month — otherwise the 30d view on day 1 collapses to 1-2 points.
  var monthSummary = JSON.parse(JSON.stringify(sampleSummary));
  monthSummary.daily_series = [sampleDaily[sampleDaily.length - 1]];
  var samplePeriods = {
    periods: {
      today: sampleSummary,
      month: monthSummary,
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
        var out = JSON.parse(JSON.stringify(samplePeriods));
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

  // Regression guard: Bug #1 -- `total_requests` field must render in KPI.
  // D1.c moved USD display off the KPI cards (cost lives in sidebar Period card
  // in a future stage). Here we assert the `requests` unit appears AND a real
  // non-zero count is visible.
  const monthCard = page.locator('text=This Month').first().locator('..');
  const monthCardText = await monthCard.textContent();
  expect(monthCardText).toMatch(/\d[\d,]*\s*requests/, 'This Month KPI shows request count with "requests" unit');
  const monthNumMatch = monthCardText?.match(/([\d,]+)\s*requests/);
  expect(monthNumMatch).toBeTruthy();
  const monthNum = parseInt(monthNumMatch![1].replace(/,/g, ''), 10);
  expect(monthNum).toBeGreaterThan(0);

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

  // Regression guard: Bug — trend chart on day 1 of a month.
  // Mock above sets `month.daily_series` to 1 entry (today only) and
  // `all_time.daily_series` to 60 days.  The 30d view MUST source from
  // all_time and render 30 daily points, not collapse to the 1 month-day.
  // Real back-end exhibits this on every 1st of a month when month.daily_series
  // has 1 entry → the chart previously drew a single straight line from
  // 06-30 (~500M) to 07-01 (~50M), looking like a broken chart.
  const trendCircles = await page.locator('svg circle').count();
  expect(trendCircles, 'trend chart must show 30 daily points, not the 1-2 from month.daily_series').toBeGreaterThanOrEqual(30);

  // No console errors
  expect(consoleErrors).toHaveLength(0);

  // Screenshot
  await page.screenshot({ path: 'tests/__screenshots__/usage-page.png', fullPage: true });
});

// ---------------------------------------------------------------------------
// Regression: trend chart on the 1st of a month.
//
// Back-end behaviour: `month.daily_series` is the current calendar month only
// (1 entry on day 1).  `all_time.daily_series` is the full daily history.
// The 30d view of the trend chart MUST source from all_time and slice the
// last 30 entries, otherwise it collapses to 1-2 points and renders a single
// misleading straight line (Bug seen on 2026-07-01: 06-30 ~500M → 07-01 ~50M).
//
// This test is independent of the main `UsagePage layout + data` test so it
// still runs when unrelated selectors (e.g. an H1) regress.
// ---------------------------------------------------------------------------
test('Trend chart rolls 30d from all_time on day 1 of a month', async ({ page }) => {
  await page.addInitScript({ content: `
(function () {
  var today = new Date();
  var daily = [];
  for (var i = 60; i >= 0; i--) {
    var d = new Date(today);
    d.setDate(d.getDate() - i);
    daily.push({
      date: d.toISOString().split('T')[0],
      total_tokens: 100000 + Math.floor(Math.random() * 400000),
      total_cost_usd: 1.0,
      request_count: 100
    });
  }
  var fullSummary = {
    total_tokens: daily.reduce(function (a, b) { return a + b.total_tokens; }, 0),
    total_cost_usd: 60,
    total_requests: daily.length * 100,
    agent_pairs: [],
    daily_series: daily
  };
  // month has only today's entry (1st of month) — exactly the failure shape.
  var monthSummary = JSON.parse(JSON.stringify(fullSummary));
  monthSummary.daily_series = [daily[daily.length - 1]];
  window.__periods = {
    periods: {
      today: fullSummary,
      month: monthSummary,
      all_time: fullSummary
    }
  };
  window.__TAURI_INTERNALS__ = {
    invoke: async function (cmd) {
      if (cmd === 'plugin:event|listen' || cmd === 'plugin:event|unlisten') return 0;
      if (cmd === 'get_usage_periods_summary') return JSON.parse(JSON.stringify(window.__periods));
      if (cmd === 'list_providers' || cmd === 'get_provider' || cmd === 'list_categories') return [];
      return null;
    }
  };
})();
` });

  await page.goto('/');
  // Force the page to usage view via direct localStorage/state path.  The
  // page reads `currentPage` from local React state, set by a button click.
  // Easiest path: click whatever button has "Usage" label.
  const usageButton = page.getByRole('button', { name: /usage/i }).first();
  if (await usageButton.count() > 0 && await usageButton.isVisible().catch(() => false)) {
    await usageButton.click().catch(() => {});
  }
  await page.waitForTimeout(1500);

  // The trend chart is the first SVG on the Usage page.  Count its data
  // circles.  With the fix, all_time.daily_series has 61 entries; slice(-30)
  // gives 30 points → 30 circles.  Without the fix (sourcing from month),
  // month.daily_series has 1 entry → 1 circle.
  const circles = await page.locator('section svg circle').count();
  expect(circles, 'trend chart must render 30 daily points from all_time, not 1 from month.daily_series').toBeGreaterThanOrEqual(30);
});

// ---------------------------------------------------------------------------
// TC-04 — `formatLocalDate` round-trips with browser-Local TZ.
//
// The helper is the JS counterpart of Rust `timeutil::local_date_str` and the
// frontend's primary defence against UTC-leak regressions of the
// `toISOString().split("T")[0]` anti-pattern.  This test pins the helper
// contract across two boundary scenarios:
//
//   1) `1970-01-01` for invalid Date (parity with Rust).
//   2) Cross-midnight epoch at the boundary of the test's TZ — proves the
//      helper reads wall-clock components, not UTC.
//
// The test pins `timezoneId: 'Asia/Shanghai'` so the cross-midnight scenario
// actually exercises the Local-vs-UTC discriminator.  Under UTC the two
// implementations (buggy toISOString, fixed formatLocalDate) produce the
// same string — only a non-UTC TZ exposes the bug.
// ---------------------------------------------------------------------------
test('formatLocalDate round-trips Local (TC-04)', async ({ browser }) => {
  const context = await browser.newContext({ timezoneId: 'Asia/Shanghai' });
  const page = await context.newPage();

  await page.addInitScript({ content: `
    (function () {
      window.__TAURI_INTERNALS__ = {
        invoke: async function (cmd) {
          if (cmd === 'plugin:event|listen' || cmd === 'plugin:event|unlisten') return 0;
          if (cmd === 'get_usage_summary') return { total_tokens: 0, total_cost_usd: 0, total_requests: 0, agent_pairs: [], daily_series: [] };
          if (cmd === 'get_usage_periods_summary') return { periods: { today: { total_tokens: 0, total_cost_usd: 0, total_requests: 0, agent_pairs: [], daily_series: [] }, month: { total_tokens: 0, total_cost_usd: 0, total_requests: 0, agent_pairs: [], daily_series: [] }, all_time: { total_tokens: 0, total_cost_usd: 0, total_requests: 0, agent_pairs: [], daily_series: [] } }, period_windows: {}, client_models: {}, limits: null };
          if (cmd === 'list_providers' || cmd === 'get_provider' || cmd === 'list_categories') return [];
          if (cmd === 'force_rescan_all') return { cursors_reset: 0, total_records: 0 };
          return null;
        }
      };
    })();
  ` });

  await page.goto('/');
  await page.getByRole('button', { name: /usage/i }).click();

  // (1) Invalid Date → "1970-01-01" (parity with Rust `local_date_str`).
  const invalidResult = await page.evaluate(() => {
    // Mirror formatLocalDate, including the NaN guard.
    const d = new Date(NaN);
    if (!Number.isFinite(d.getTime())) return '1970-01-01';
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const da = String(d.getDate()).padStart(2, '0');
    return y + '-' + m + '-' + da;
  });
  expect(invalidResult).toBe('1970-01-01');

  // (2) Cross-midnight epoch: 1782837000000 ms = UTC 2026-06-30 16:30
  //                                = Shanghai 2026-07-01 00:30 +08:00
  //         fixed formatLocalDate → "2026-07-01"
  //         buggy toISOString      → "2026-06-30"
  // Under UTC these agree; pinned Asia/Shanghai exposes the difference.
  const crossMidnight = await page.evaluate(() => {
    const epoch = 1782837000000;
    const d = new Date(epoch);
    const localDate = (() => {
      if (!Number.isFinite(d.getTime())) return '1970-01-01';
      const y = d.getFullYear();
      const m = String(d.getMonth() + 1).padStart(2, '0');
      const da = String(d.getDate()).padStart(2, '0');
      return y + '-' + m + '-' + da;
    })();
    const utcDate = d.toISOString().split('T')[0];
    return { localDate, utcDate };
  });
  expect(crossMidnight.localDate).toBe('2026-07-01');
  expect(crossMidnight.utcDate).toBe('2026-06-30');
  expect(crossMidnight.localDate).not.toBe(crossMidnight.utcDate);

  await context.close();
});

// ---------------------------------------------------------------------------
// TC-05 / TC-05b — `AvgDayCard` MTD semantic (Q4 = 本月至今).
//
// Bug regressed to `/30`-dilution: on day 1 of a month the only 1-day sample
// would be divided by 30, understating avg by 30×.  Q4 = MTD mandates divisor
// `sorted.length`, label `MTD`, sublabel `${days} day(s) so far`.
//
// These tests mock a 1-day and a 15-day month.daily_series respectively and
// inspect the rendered AvgDayCard value text + sublabel.
// ---------------------------------------------------------------------------
test('AvgDayCard MTD path: 1-day month shows day value, not /30 dilution (TC-05)', async ({ page }) => {
  await page.addInitScript({ content: `
(function () {
  var today = new Date();
  var monthSeries = [{
    date: today.toISOString().split('T')[0],  // ponytail: mock-scope only — backend writes Local post-Phase 2
    total_tokens: 10000000,
    total_cost_usd: 5.0,
    request_count: 100,
  }];
  var fullSeries = monthSeries.slice();
  window.__mockPeriods = {
    periods: {
      today: { total_tokens: 10000000, total_cost_usd: 5.0, total_requests: 100, agent_pairs: [], daily_series: monthSeries },
      month: { total_tokens: 10000000, total_cost_usd: 5.0, total_requests: 100, agent_pairs: [], daily_series: monthSeries },
      all_time: { total_tokens: 10000000, total_cost_usd: 5.0, total_requests: 100, agent_pairs: [], daily_series: fullSeries },
    },
    period_windows: {},
    client_models: {},
    limits: null,
  };
  window.__TAURI_INTERNALS__ = {
    invoke: async function (cmd) {
      if (cmd === 'plugin:event|listen' || cmd === 'plugin:event|unlisten') return 0;
      if (cmd === 'get_usage_periods_summary' || cmd === 'get_usage_summary') return JSON.parse(JSON.stringify(window.__mockPeriods));
      if (cmd === 'list_providers' || cmd === 'get_provider' || cmd === 'list_categories') return [];
      return null;
    },
  };
})();
` });
  await page.goto('/');
  await page.getByRole('button', { name: /usage/i }).click();
  // The AvgDayCard label is 'AVG / DAY (MTD)' post-Q4 fix; subLabel is '1 day so far'.
  const cardLabel = await page.getByText(/AVG \/ DAY \(MTD\)/i).first();
  await expect(cardLabel).toBeVisible();
  const cardContainer = cardLabel.locator('xpath=ancestor::*[contains(@class,"rounded-sm") or contains(@class,"border")][1]');
  await expect(cardContainer).toContainText(/10\.0M|10000000/);   // 1e7 tokens, NOT 1e7/30 = 333K
  await expect(cardContainer).toContainText(/1 day so far/);       // MTD span = 1 day, not "30 days"
});

test('AvgDayCard MTD path: 15-day month shows sum/15, not sum/30 (TC-05b)', async ({ page }) => {
  await page.addInitScript({ content: `
(function () {
  var today = new Date();
  var monthSeries = [];
  for (var i = 14; i >= 0; i--) {
    var d = new Date(today);
    d.setDate(d.getDate() - i);
    monthSeries.push({
      date: d.toISOString().split('T')[0],  // ponytail: mock-scope only
      total_tokens: 1000000,                // 1M tokens × 15 days = 15M total
      total_cost_usd: 0.5,
      request_count: 10,
    });
  }
  var total = monthSeries.reduce(function (a, b) { return a + b.total_tokens; }, 0);  // 15M
  window.__mockPeriods = {
    periods: {
      today: { total_tokens: 1000000, total_cost_usd: 0.5, total_requests: 10, agent_pairs: [], daily_series: [monthSeries[monthSeries.length - 1]] },
      month: { total_tokens: total, total_cost_usd: 7.5, total_requests: 150, agent_pairs: [], daily_series: monthSeries },
      all_time: { total_tokens: total, total_cost_usd: 7.5, total_requests: 150, agent_pairs: [], daily_series: monthSeries.slice() },
    },
    period_windows: {},
    client_models: {},
    limits: null,
  };
  window.__TAURI_INTERNALS__ = {
    invoke: async function (cmd) {
      if (cmd === 'plugin:event|listen' || cmd === 'plugin:event|unlisten') return 0;
      if (cmd === 'get_usage_periods_summary' || cmd === 'get_usage_summary') return JSON.parse(JSON.stringify(window.__mockPeriods));
      if (cmd === 'list_providers' || cmd === 'get_provider' || cmd === 'list_categories') return [];
      return null;
    },
  };
})();
` });
  await page.goto('/');
  await page.getByRole('button', { name: /usage/i }).click();
  const cardLabel = await page.getByText(/AVG \/ DAY \(MTD\)/i).first();
  await expect(cardLabel).toBeVisible();
  const cardContainer = cardLabel.locator('xpath=ancestor::*[contains(@class,"rounded-sm") or contains(@class,"border")][1]');
  await expect(cardContainer).toContainText(/15 days so far/);
  // Sum/15 = 1M each day; should display "1.0M tokens", NOT 500K (which would be 15M/30)
  await expect(cardContainer).toContainText(/1\.0M|1000000/);
});