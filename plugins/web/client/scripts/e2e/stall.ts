import { spawnSync } from "node:child_process";
import { assert, type Cdp } from "./cdp";
import { selectTerminal, takeControl } from "./basic";

/** Pauses the plugin with an operator command and proves the tab reconnects itself. */
export async function stall(
  page: Cdp,
  terminalId: string,
  pause: string,
  resume: string,
): Promise<void> {
  await selectTerminal(page, terminalId);
  await takeControl(page);
  const paused = spawnSync(pause, { shell: true, stdio: "inherit" });
  assert(paused.status === 0, `pause command failed: ${paused.status}`);
  try {
    await page.waitFor(
      "document.getElementById('terminal-input-state')?.textContent === 'Connecting…'",
      70_000,
    );
  } finally {
    const resumed = spawnSync(resume, { shell: true, stdio: "inherit" });
    assert(resumed.status === 0, `resume command failed: ${resumed.status}`);
  }
  await page.waitFor(
    "!document.getElementById('terminal-input-state')?.textContent?.includes('Connecting')",
    30_000,
  );
  await takeControl(page);
  const marker = `STALL-${Date.now()}`;
  await page.evaluate("document.querySelector('.xterm-helper-textarea')?.focus()");
  await page.type(`echo ${marker}`);
  await page.enter();
  await page.waitFor(
    `document.querySelector('.xterm-rows')?.textContent?.includes(${JSON.stringify(marker)})`,
    15_000,
  );
}
