---
name: playwright
description: "Drive headless Chromium/Firefox/WebKit with Microsoft Playwright for scripted browser automation, end-to-end tests, screenshots, PDF print, and scrapers that do not need the user's logged-in Chrome profile. Prefer the browser-use skill when the task must run inside the user's real Chrome with existing cookies. Triggers: playwright, headless browser, e2e test, browser automation script, scrape without login, screenshot page, print pdf."
license: Apache-2.0
metadata:
  third_party:
    - kind: library
      name: Playwright
      info_url: https://github.com/microsoft/playwright
      terms_url: https://github.com/microsoft/playwright/blob/main/LICENSE
---

# Playwright — headless browser automation

Use **Playwright** when you need a disposable automation browser (Chromium / Firefox / WebKit) controlled by a script. It is **not** the same as SuperScience's `browser-use` skill:

| Need | Use |
|---|---|
| User's real Chrome, cookies, logged-in sites | `browser-use` |
| Headless script, CI-style e2e, scrape without user profile | **this skill (`playwright`)** |

Upstream: https://github.com/microsoft/playwright (Apache-2.0). This bundled skill documents the CLI/API; it does **not** vendor the full Playwright monorepo into the DMG.

## Setup (once per machine / project)

Prefer a project-local install so versions stay pinned:

```bash
# Node.js >= 18 required
npm init -y
npm install -D playwright @playwright/test
npx playwright install chromium
```

If only Chromium is needed (smaller download):

```bash
npx playwright install chromium
```

Verify:

```bash
npx playwright --version
node -e "const {chromium}=require('playwright'); chromium.launch().then(b=>b.close()).then(()=>console.log('ok'))"
```

On air-gapped or failed browser downloads, stop and tell the user to install browsers with `npx playwright install`, rather than inventing a different stack.

## Preferred workflow

1. Clarify goal: scrape, screenshot, fill form, assert UI, export PDF.
2. Confirm Node + Playwright are available (commands above). Prefer `browser-use` if they need an already-logged-in site.
3. Write a small script under the project (e.g. `scripts/playwright_task.mjs`) and run it with `node` / `npx playwright test`.
4. Prefer **role / text / test-id locators** over brittle CSS; wait for network idle or a specific selector before acting.
5. Save screenshots/PDFs into the project workspace and report absolute paths.

## Minimal Chromium snippet

```js
// scripts/playwright_open.mjs
import { chromium } from 'playwright';

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
await page.goto(process.argv[2] || 'https://example.com', { waitUntil: 'networkidle' });
await page.screenshot({ path: 'playwright-shot.png', fullPage: true });
console.log(await page.title());
await browser.close();
```

```bash
node scripts/playwright_open.mjs 'https://example.com'
```

## Testing with @playwright/test

```bash
npx playwright test
npx playwright show-report
```

Keep tests deterministic: fixed waits only as a last resort; assert visible text; isolate storage state when auth is required (store storageState JSON in the project, never commit secrets).

## Hard rules

- Do **not** download the entire Playwright GitHub monorepo into this skill directory.
- Do **not** replace `browser-use` for tasks that need the user's real session.
- Never paste passwords into scripts committed to the repo; use env vars or storageState files that stay local.
- Prefer Chinese replies when the UI locale / user is Chinese, unless the user asks for English.
