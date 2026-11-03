import { assert, Cdp, type Cdp as CdpType, unlock } from "./cdp";
import { selectTerminal, takeControl } from "./basic";

export type TakeoverOptions = {
  browser: CdpType;
  terminalId: string;
  url: string;
  token?: string;
};

/** Two tabs share one actor; explicit takeover demotes the first controller. */
export async function takeover(page: CdpType, options: TakeoverOptions): Promise<void> {
  await selectTerminal(page, options.terminalId);
  await takeControl(page);

  const created = await options.browser.call("Target.createTarget", { url: options.url });
  assert(Boolean(created?.targetId), "second tab was not created");
  const base = new URL(process.env.YOLO_WEB_E2E_CDP as string);
  const secondUrl = `${base.protocol}//${base.host}/devtools/page/${created.targetId}`;
  const second = await Cdp.connect(secondUrl);
  try {
    await second.waitFor(
      "document.readyState==='complete' && !!document.getElementById('app')",
      10_000,
    );
    await unlock(second, options.token);
    await selectTerminal(second, options.terminalId);
    await second.waitFor(
      "document.getElementById('terminal-input-state')?.textContent?.includes('another tab controls')",
      10_000,
    );
    await second.click("#terminal-take");
    await second.waitFor(
      "document.getElementById('terminal-input-state')?.textContent?.includes('Typing enabled')",
      10_000,
    );
    await page.waitFor(
      "document.getElementById('terminal-input-state')?.textContent?.includes('another tab controls')",
      10_000,
    );
  } finally {
    second.close();
    await options.browser.call("Target.closeTarget", { targetId: created.targetId });
  }

  await page.click("#terminal-take");
  await page.waitFor(
    "document.getElementById('terminal-input-state')?.textContent?.includes('Typing enabled')",
    10_000,
  );
  assert(page.errors.length === 0, `browser errors: ${page.errors.join("; ")}`);
}
