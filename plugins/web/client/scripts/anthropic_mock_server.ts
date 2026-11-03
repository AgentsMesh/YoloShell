import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { startAnthropicMock } from "./e2e/anthropicMock";

const root = process.env.YOLO_WEB_E2E_MOCK_ROOT ?? "/tmp/yolo-web-conversation";
const reads = Number(process.env.YOLO_WEB_E2E_CONVERSATION_READS ?? "40");
const port = Number(process.env.YOLO_WEB_E2E_MOCK_PORT ?? "8791");
const hostname = process.env.YOLO_WEB_E2E_MOCK_HOST ?? "0.0.0.0";
const dir = join(root, "files");
const allow = join(root, "allow");
const deny = join(root, "deny");

rmSync(root, { recursive: true, force: true });
mkdirSync(dir, { recursive: true });
for (let index = 1; index <= reads; index += 1) writeFileSync(join(dir, `f${index}.txt`), `file ${index}\n`);
const mock = startAnthropicMock({ reads, dir, allow, deny, hostname, port });
writeFileSync("/tmp/yolo-web-conversation-mock.pid", String(process.pid));
console.log(`mock listening on http://${hostname}:${mock.port}`);
process.on("SIGTERM", () => {
  mock.stop();
  process.exit(0);
});
await new Promise(() => {});
