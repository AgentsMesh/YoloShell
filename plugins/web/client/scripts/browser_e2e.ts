/**
 * Browser acceptance for the Web client. It drives an already-running plugin
 * through Chrome DevTools; it is not a unit test.
 *
 *   YOLO_WEB_E2E_CDP=ws://127.0.0.1:9222/devtools/page/<id> \
 *   YOLO_WEB_E2E_TERMINAL=<disposable-shell-id> \
 *   [YOLO_WEB_E2E_AGENT=<alternate-screen-agent-id>] \
 *   [YOLO_WEB_E2E_TOKEN=<plugin-token>] \
 *   [YOLO_WEB_E2E_SCENARIO=basic|creation|takeover|stall|all] \
 *   [YOLO_WEB_E2E_BROWSER_CDP=ws://127.0.0.1:9222/devtools/browser/<id>] \
 *   [YOLO_WEB_E2E_PAUSE='kill -STOP <pid>'] \
 *   [YOLO_WEB_E2E_RESUME='kill -CONT <pid>'] \
 *   bun scripts/browser_e2e.ts
 */

import { Cdp, assert, reload, unlock } from "./e2e/cdp";
import { basic } from "./e2e/basic";
import { creation } from "./e2e/creation";
import { stall } from "./e2e/stall";
import { takeover } from "./e2e/takeover";

const cdp = process.env.YOLO_WEB_E2E_CDP;
const terminalId = process.env.YOLO_WEB_E2E_TERMINAL;
const scenario = process.env.YOLO_WEB_E2E_SCENARIO ?? "basic";
const token = process.env.YOLO_WEB_E2E_TOKEN;
if (!cdp || !terminalId) {
  throw new Error("YOLO_WEB_E2E_CDP and YOLO_WEB_E2E_TERMINAL are required");
}

const page = await Cdp.connect(cdp);
try {
  await reload(page);
  await unlock(page, token);
  await page.waitFor("document.getElementById('hub-status')?.textContent?.trim().length > 0", 15_000);
  if (scenario === "basic" || scenario === "all") {
    await basic(page, {
      terminalId,
      agentId: process.env.YOLO_WEB_E2E_AGENT,
    });
  }
  if (scenario === "creation" || scenario === "all") {
    await creation(page);
  }
  if (scenario === "takeover" || scenario === "all") {
    const browserCdp = process.env.YOLO_WEB_E2E_BROWSER_CDP;
    assert(Boolean(browserCdp), "YOLO_WEB_E2E_BROWSER_CDP is required for takeover");
    const browser = await Cdp.connect(browserCdp!, false);
    try {
      const url = await page.evaluate("location.origin + '/'");
      await takeover(page, { browser, terminalId, url, token });
    } finally {
      browser.close();
    }
  }
  if (scenario === "stall" || scenario === "all") {
    const pause = process.env.YOLO_WEB_E2E_PAUSE;
    const resume = process.env.YOLO_WEB_E2E_RESUME;
    assert(Boolean(pause && resume), "YOLO_WEB_E2E_PAUSE and YOLO_WEB_E2E_RESUME are required for stall");
    await stall(page, terminalId, pause!, resume!);
  }
  assert(page.errors.length === 0, `browser errors: ${page.errors.join("; ")}`);
  console.log(`web browser e2e passed: ${scenario}`);
} finally {
  page.close();
}
