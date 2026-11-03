import { expect, test } from "bun:test";
import { fileURLToPath } from "node:url";

const cdp = process.env.YOLO_WEB_E2E_CDP;
const terminal = process.env.YOLO_WEB_E2E_TERMINAL;

test.skipIf(!cdp || !terminal)(
  "browser e2e smoke",
  async () => {
    const root = fileURLToPath(new URL("..", import.meta.url));
    const proc = Bun.spawn([process.execPath, "scripts/browser_e2e.ts"], {
      cwd: root,
      env: process.env,
      stdout: "pipe",
      stderr: "pipe",
    });
    const [stdout, stderr, code] = await Promise.all([
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
      proc.exited,
    ]);
    if (code !== 0) throw new Error(`browser e2e failed (${code}): ${stderr || stdout}`);
    expect(stdout).toContain("web browser e2e passed");
  },
  300_000,
);
