import { dirname } from "node:path";
import { Cdp, assert, reload, unlock } from "./e2e/cdp";

const cdp = process.env.YOLO_WEB_E2E_CDP;
const token = process.env.YOLO_WEB_E2E_TOKEN;
const claude = process.env.YOLO_WEB_E2E_CLAUDE_BIN;
const claudeHome = process.env.YOLO_WEB_E2E_CLAUDE_HOME;
const settings = process.env.YOLO_WEB_E2E_CLAUDE_SETTINGS;
const mockUrl = process.env.YOLO_WEB_E2E_MOCK_URL;
const vm = process.env.YOLO_WEB_E2E_VM ?? "";
const model = process.env.YOLO_WEB_E2E_CLAUDE_MODEL ?? "claude-sonnet-4-5";
assert(
  Boolean(cdp && token && claude && claudeHome && settings && mockUrl),
  "CDP, token, mock URL and Claude env are required",
);

type MockState = { steps: number; reads: number; allow: boolean; deny: boolean };
async function mockState(): Promise<MockState> {
  const response = await fetch(`${mockUrl}/state`);
  assert(response.ok, `mock state request failed: ${response.status}`);
  return (await response.json()) as MockState;
}

function cleanup(terminal: string): void {
  if (!terminal) return;
  const script = `yolo terminate ${terminal} >/dev/null 2>&1 || true; sleep 1; yolo archive ${terminal} >/dev/null 2>&1 || true`;
  const args = vm
    ? ["orb", "-m", vm, "-u", process.env.YOLO_WEB_E2E_VM_USER ?? process.env.USER ?? "stone", "bash", "-lc", script]
    : ["bash", "-lc", script];
  Bun.spawnSync(args, { stdout: "ignore", stderr: "ignore" });
}

const page = await Cdp.connect(cdp!);
const text = (selector: string) =>
  page.evaluate(`document.querySelector(${JSON.stringify(selector)})?.textContent?.trim().replace(/\\s+/g,' ').slice(0,500) ?? ''`);
const count = (selector: string) => page.evaluate(`document.querySelectorAll(${JSON.stringify(selector)}).length`);
const enter = async () => {
  const key = { key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13, text: "\r", unmodifiedText: "\r" };
  await page.call("Input.dispatchKeyEvent", { type: "keyDown", ...key });
  await page.call("Input.dispatchKeyEvent", { type: "keyUp", ...key });
};
let terminal = "";
try {
  const initial = await mockState();
  assert(!initial.allow && !initial.deny, "mock markers are dirty");
  await page.call("Emulation.setDeviceMetricsOverride", { width: 1600, height: 950, deviceScaleFactor: 1, mobile: false });
  await reload(page);
  await unlock(page, token);
  await page.waitFor("document.getElementById('hub-status')?.textContent?.trim().length > 0", 15_000);
  const name = `conv-e2e-${Date.now()}`;
  await page.click("#new-shell");
  await page.waitFor("document.getElementById('create')?.open === true", 5_000);
  await page.fill("#create-cwd", "/tmp");
  await page.fill("#create-name", name);
  await page.click("#create-submit");
  await page.waitFor("document.getElementById('create')?.open === false", 30_000);
  await page.waitFor(`(()=>{const b=[...document.querySelectorAll('#terminal-list button[data-id]')].find(e=>e.textContent.includes(${JSON.stringify(name)})); return !!b})()`, 30_000);
  terminal = await page.evaluate(`(()=>{const b=[...document.querySelectorAll('#terminal-list button[data-id]')].find(e=>e.textContent.includes(${JSON.stringify(name)})); return b?.dataset.id ?? ''})()`);
  assert(Boolean(terminal), "created terminal not found");
  await page.waitFor("document.getElementById('terminal-input-state')?.textContent?.includes('Typing enabled')", 30_000);
  await page.evaluate("document.querySelector('.xterm-helper-textarea')?.focus()");
  await page.type(`env HOME=${claudeHome} PATH=${dirname(claude!)}:/usr/local/bin:/usr/bin:/bin ANTHROPIC_BASE_URL=${mockUrl} ANTHROPIC_AUTH_TOKEN=e2e ANTHROPIC_MODEL=${model} CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 DISABLE_TELEMETRY=1 ${claude} --settings ${settings}`);
  await enter();
  await page.waitFor("document.querySelector('.xterm-rows')?.textContent?.includes('Claude Code')", 20_000);
  await Bun.sleep(500);
  await page.type("Run the checks");
  await enter();
  await page.waitFor(`(()=>{const b=[...document.querySelectorAll('#needs-list button')].find(e=>e.textContent.includes(${JSON.stringify(name)})); return !!b})()`, 120_000);
  const agent = await page.evaluate(`(()=>{const b=[...document.querySelectorAll('#agent-list button[data-kind=agent]')].find(e=>e.textContent.includes(${JSON.stringify(name)})); return b?.dataset.id ?? ''})()`);
  assert(Boolean(agent), "Claude agent bound without a row");
  await page.evaluate(`(()=>{const b=[...document.querySelectorAll('#needs-list button')].find(e=>e.textContent.includes(${JSON.stringify(name)})); b?.click();})()`);
  await page.waitFor("document.getElementById('interaction')?.hidden === false", 10_000);
  assert((await text("#interaction-prompt")).includes("allow"), "allow interaction prompt missing");
  assert((await count("#interaction-choices button[data-choice=allow]")) === 1, "allow choice missing");
  assert((await count("#interaction-choices button[data-choice=deny]")) === 1, "deny choice missing");
  await page.click("#interaction-choices button[data-choice=allow]");
  await page.waitFor("document.getElementById('interaction')?.hidden === true", 20_000);
  for (let attempt = 0; attempt < 30 && !(await mockState()).allow; attempt += 1) await Bun.sleep(500);
  assert((await mockState()).allow, "allowed command did not run");
  await page.waitFor("document.getElementById('interaction')?.hidden === false", 300_000);
  assert((await text("#interaction-prompt")).includes("deny"), "deny interaction prompt missing");
  await page.click("#interaction-choices button[data-choice=deny]");
  await page.waitFor("document.getElementById('interaction')?.hidden === true", 20_000);
  await page.waitFor("document.getElementById('turns')?.textContent?.includes('CONVERSATION_DONE')", 120_000);
  const final = await mockState();
  assert(!final.deny, "denied command ran");
  assert(final.steps >= final.reads + 3, `mock stopped early at ${final.steps}`);
  await page.waitFor("document.getElementById('older')?.hidden === false", 30_000);
  const before = await count("#turns .turn");
  assert(before === 64, `latest conversation page was ${before} turns`);
  await page.click("#older");
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if ((await count("#turns .turn")) > before) break;
    await Bun.sleep(200);
  }
  const after = await count("#turns .turn");
  assert(after > before, "older conversation page did not load");
  const note = await text("#conversation-note");
  assert(!note.includes("Some history is unavailable"), `partial history note: ${note}`);
  assert((await text("#turns .turn"))?.includes("Run the checks"), "older page does not start at the prompt");
  assert(await page.evaluate("document.getElementById('older')?.hidden === true"), "older button should be exhausted");
  console.log(`web conversation e2e passed: ${before} → ${after} turns, ${final.steps} mock steps`);
} finally {
  try { await page.call("Emulation.clearDeviceMetricsOverride"); } catch {}
  page.close();
  cleanup(terminal);
}
