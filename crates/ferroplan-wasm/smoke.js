// Browser smoke test for the wasm demo pages (0.18 Phase 3): the solver
// page's introspection views (causal chain, invariant spans) and the
// village live page's first ticks (both workers think, economy panel
// live). This caught a real one on arrival: the 32-bit node-cap wrap
// that had silently killed every default-cap wasm solve (all of
// temporal) since 0.8 — run it before every cut.
//
//   cd crates/ferroplan-wasm && ./build.sh    # fresh pkg/ first
//   npm i playwright                          # any scratch dir works
//   node smoke.js
//
// Playwright finds its browser via PLAYWRIGHT_BROWSERS_PATH, with the
// /opt/pw-browsers/chromium executablePath as the pinned-env fallback.
const { chromium } = require('playwright');
const { spawn } = require('child_process');

const path = require('path');
const WEB = path.join(__dirname, 'web');
const REPO = path.join(__dirname, '..', '..');
const PORT = 8931;

(async () => {
  const server = spawn('python3', ['-m', 'http.server', String(PORT), '-d', WEB]);
  await new Promise(r => setTimeout(r, 1200));
  let browser;
  try {
    browser = await chromium.launch({ executablePath: '/opt/pw-browsers/chromium' }).catch(() =>
      chromium.launch());
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', e => errors.push('pageerror: ' + e.message));
    page.on('console', m => { if (m.type() === 'error') errors.push('console: ' + m.text()); });

    // ---- 1. solver page: solve gripper, explain -> causal chain ----
    await page.goto(`http://127.0.0.1:${PORT}/index.html`);
    await page.waitForFunction(() => document.getElementById('goLabel').textContent === 'Plan', { timeout: 30000 });
    await page.selectOption('#exec', 'main');
    await page.click('#go');
    await page.waitForSelector('#explainBtn', { timeout: 60000 });
    await page.click('#explainBtn');
    await page.waitForFunction(() =>
      document.getElementById('explain').textContent.includes('Causal chain'), { timeout: 60000 });
    const causal = await page.textContent('#explain');
    if (!/initial state|step \d\d/.test(causal)) throw new Error('causal chain has no provider links');
    console.log('OK solver: causal chain rendered with provider links');

    // ---- 2. solver page: temporal fixture -> invariant spans ----
    const fs = require('fs');
    const epsDom = fs.readFileSync(path.join(REPO, 'benchmarks/bench/eps-cross-domain.pddl'), 'utf8');
    const epsPrb = fs.readFileSync(path.join(REPO, 'benchmarks/bench/eps-cross-p01.pddl'), 'utf8');
    await page.evaluate(([d, p]) => {
      const set = (id, v) => {
        const ta = document.getElementById(id);
        ta.value = v; ta.dispatchEvent(new Event('input'));
      };
      set('dom', d); set('prob', p);
    }, [epsDom, epsPrb]);
    await page.click('#go');
    await page.waitForSelector('#explainBtn', { timeout: 120000 });
    await page.click('#explainBtn');
    await page.waitForFunction(() =>
      document.getElementById('explain').textContent.includes('Invariant spans'), { timeout: 60000 });
    const spans = await page.textContent('#explain');
    if (!/over all: .*LIGHT.*M1/i.test(spans)) throw new Error('invariant span conditions missing: ' + spans.slice(0, 200));
    console.log('OK solver: invariant spans rendered with over-all conditions');

    // ---- 3. village live page: ticks, thinks, map, economy ----
    await page.goto(`http://127.0.0.1:${PORT}/village-live.html`);
    await page.waitForFunction(() =>
      document.getElementById('status').textContent.includes('live'), { timeout: 60000 });
    const stations = await page.locator('#map circle.stn').count();
    if (stations !== 6) throw new Error('map has ' + stations + ' stations, want 6');
    // step until both workers have thought once (first ticks include the
    // bounded thinks — give them room)
    for (let i = 0; i < 6; i++) {
      await page.click('#b-step');
      await page.waitForFunction(() => !document.getElementById('b-step').disabled, { timeout: 300000 });
    }
    await page.waitForFunction(() => {
      const t = document.getElementById('feed').textContent;
      return /bob: planned \d+ steps/.test(t) && /mara: planned \d+ steps/.test(t);
    }, { timeout: 300000 });
    const feed = await page.textContent('#feed');
    console.log('OK village: both workers planned;',
      'feed has', (feed.match(/planned/g) || []).length, 'thinks after 6 ticks');
    const econ = await page.textContent('#econ');
    if (!econ.includes('money')) throw new Error('economy panel missing');
    console.log('OK village: economy panel live');

    // resource-load noise (fonts blocked offline, favicon 404) is not a
    // page defect — only script errors count
    const fatal = errors.filter(e => !/favicon|Failed to load resource/.test(e));
    if (fatal.length) throw new Error('page errors:\n' + fatal.join('\n'));
    console.log('SMOKE PASS');
  } finally {
    if (browser) await browser.close();
    server.kill();
  }
})().catch(e => { console.error('SMOKE FAIL:', e.message); process.exit(1); });
