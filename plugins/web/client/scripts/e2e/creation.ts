import { assert, type Cdp } from "./cdp";
import { selectTerminal, takeControl } from "./basic";

/** Creates a disposable shell through the UI, ends it and archives it from History. */
export async function creation(page: Cdp): Promise<string> {
  const name = `e2e-${Date.now()}`;
  await page.click("#new-shell");
  await page.waitFor("document.getElementById('create')?.open === true");
  await page.fill("#create-cwd", "/tmp");
  await page.fill("#create-name", name);
  await page.click("#create-submit");
  await page.waitFor("document.getElementById('create')?.open === false", 30_000);
  await page.waitFor(
    `(()=>{const b=[...document.querySelectorAll('#terminal-list button[data-id]')].find(e=>e.textContent.includes(${JSON.stringify(name)})); return !!b})()`,
    20_000,
  );

  const id = await page.evaluate(
    `(()=>{const b=[...document.querySelectorAll('#terminal-list button[data-id]')].find(e=>e.textContent.includes(${JSON.stringify(name)})); return b?.dataset.id ?? null})()`,
  );
  assert(Boolean(id), `created shell row ${name} not found`);
  await page.waitFor(
    `document.querySelector('#terminal-list button[data-id=${JSON.stringify(id)}]')?.classList.contains('is-selected') === true`,
    20_000,
  );
  await selectTerminal(page, id);
  await takeControl(page);
  await page.evaluate("document.querySelector('.xterm-helper-textarea')?.focus()");
  await page.type("exit");
  await page.enter();
  await page.waitFor(
    `!document.querySelector('#terminal-list button[data-id=${JSON.stringify(id)}]')`,
    20_000,
  );

  await page.click("#archive-open");
  await page.waitFor("document.querySelectorAll('#archive-list button').length > 0");
  const before = await page.evaluate("document.getElementById('archive-note')?.textContent");
  const found = await page.evaluate(
    `(()=>{const b=[...document.querySelectorAll('#archive-list button')].find(e=>e.textContent.includes(${JSON.stringify(name)})); b?.click(); return !!b})()`,
  );
  assert(found, `ended row ${name} not found in History`);
  await page.waitFor("document.getElementById('archive-finish')?.hidden === false", 5_000);
  await page.click("#archive-finish");
  await page.waitFor("document.getElementById('archive-finish')?.hidden === true", 15_000);
  const after = await page.evaluate("document.getElementById('archive-note')?.textContent");
  assert(before !== after, "archive count did not change");
  await page.click("#archive-close");
  return id;
}
