import { expect, test } from "bun:test";
import { fileURLToPath } from "node:url";

const cdp = process.env.YOLO_WEB_E2E_CDP;
const token = process.env.YOLO_WEB_E2E_TOKEN;
const claude = process.env.YOLO_WEB_E2E_CLAUDE_BIN;

test.skipIf(!cdp || !token || !claude)(
  "typed approval and conversation paging",
  async () => {
    const root = fileURLToPath(new URL("..", import.meta.url));
    const proc = Bun.spawn([process.execPath, "scripts/conversation_e2e.ts"], {
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
    if (code !== 0) throw new Error(`conversation e2e failed (${code}): ${stderr || stdout}`);
    expect(stdout).toContain("web conversation e2e passed");
  },
  900_000,
);
