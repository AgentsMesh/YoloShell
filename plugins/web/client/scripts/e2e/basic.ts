import { assert, type Cdp } from "./cdp";

export type BasicOptions = {
  terminalId: string;
  agentId?: string;
};

export async function selectTerminal(page: Cdp, terminalId: string): Promise<void> {
  // A freshly unlocked tab loads the catalogue asynchronously; clicking a
  // missing row would silently keep the auto-selected terminal.
  await page.waitFor(
    `!!document.querySelector('button[data-id=${JSON.stringify(terminalId)}]')`,
    15_000,
  );
  await page.evaluate(
    `document.querySelector('button[data-id=${JSON.stringify(terminalId)}]')?.click()`,
  );
  await page.waitFor("!!document.querySelector('#terminal-host .xterm')", 10_000);
}

export async function takeControl(page: Cdp): Promise<void> {
  await page.click("#terminal-host");
  await page.waitFor(
    "document.getElementById('terminal-input-state')?.textContent?.includes('Typing enabled')",
    10_000,
  );
}

export async function basic(page: Cdp, options: BasicOptions): Promise<void> {
  await page.click("#archive-open");
  await page.waitFor("document.querySelectorAll('#archive-list button').length > 0");
  const history = await page.evaluate(
    "(()=>{const l=document.getElementById('archive-list'); l.scrollTop=0; return {scrollHeight:l.scrollHeight, clientHeight:l.clientHeight}})()",
  );
  assert(history.scrollHeight > history.clientHeight, "History list has no scroll range");
  await page.wheel("#archive-list", 500);
  await page.waitFor("document.getElementById('archive-list').scrollTop > 0");
  await page.click("#archive-close");

  await selectTerminal(page, options.terminalId);
  await takeControl(page);
  await page.evaluate("document.querySelector('.xterm-helper-textarea')?.focus()");
  const marker = `E2E-${Date.now()}`;
  await page.type(`echo ${marker}`);
  await page.enter();
  await page.waitFor(
    `document.querySelector('.xterm-rows')?.textContent?.includes(${JSON.stringify(marker)})`,
    10_000,
  );

  await page.click("#terminal-log");
  await page.waitFor(
    "!document.getElementById('terminal-log-note')?.textContent?.startsWith('Loading')",
  );
  const log = await page.evaluate(
    "(()=>{const p=document.getElementById('terminal-log-text'); return {length:p.textContent.length, scrollHeight:p.scrollHeight, clientHeight:p.clientHeight}})()",
  );
  assert(log.length > 0, "terminal log is empty");
  if (log.scrollHeight > log.clientHeight) {
    await page.evaluate("document.getElementById('terminal-log-text').scrollTop=0");
    await page.wheel("#terminal-log-text", 400);
    await page.waitFor("document.getElementById('terminal-log-text').scrollTop > 0");
  }
  await page.evaluate("document.getElementById('terminal-log-close').click()");

  if (options.agentId) {
    await selectTerminal(page, options.agentId);
    await page.click("#lens-terminal");
    await page.waitFor("document.getElementById('terminal-view')?.hidden === false");
    await page.click("#terminal-log");
    await page.waitFor(
      "!document.getElementById('terminal-log-note')?.textContent?.startsWith('Loading')",
    );
    const agentLog = await page.evaluate(
      "(()=>{const p=document.getElementById('terminal-log-text'); return {length:p.textContent.length, scrollHeight:p.scrollHeight, clientHeight:p.clientHeight}})()",
    );
    assert(agentLog.length > 0, "agent terminal log is empty");
    if (agentLog.scrollHeight > agentLog.clientHeight) {
      await page.evaluate("document.getElementById('terminal-log-text').scrollTop=0");
      await page.wheel("#terminal-log-text", 400);
      await page.waitFor("document.getElementById('terminal-log-text').scrollTop > 0");
    }
    await page.click("#terminal-log-close");
  }
  assert(page.errors.length === 0, `browser errors: ${page.errors.join("; ")}`);
}
