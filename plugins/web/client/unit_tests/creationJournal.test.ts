import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { CreationIntentStore, type CreationIntent } from "../server/creationIntent";
import { prepare, spec } from "../server/creationRequest";
import { runIntent } from "../server/creationRunner";

const HUB = crypto.randomUUID();
process.env.YOLO_HUB_ID = HUB;
process.env.YOLO_HUB_SOCKET = "/tmp/yolo-web-journal-test.sock";

function directory(): string {
  return mkdtempSync(join(tmpdir(), "yolo-web-intents-"));
}

function intent(data: string, state: CreationIntent["state"] = "prepared"): CreationIntent {
  const prepared = prepare(spec("shell", { cwd: "/tmp", name: "journal" }));
  const now = Date.now();
  return {
    version: 1,
    id: crypto.randomUUID(),
    idempotency_key: crypto.randomUUID().replaceAll("-", ""),
    fingerprint: "fingerprint",
    kind: "shell",
    origin: prepared.origin,
    request: prepared.request,
    state,
    attempts: 0,
    createdAt: now,
    updatedAt: now,
  };
}

describe("creation intent journal", () => {
  test("persists a private record before dispatch and reloads it", () => {
    const data = directory();
    const store = new CreationIntentStore(data);
    const record = intent(data);
    store.insert(record);

    const path = join(data, "creation-intents", `${record.id}.json`);
    expect(statSync(path).mode & 0o777).toBe(0o600);
    expect(JSON.parse(readFileSync(path, "utf8")).request).toEqual(record.request);

    const reloaded = new CreationIntentStore(data);
    reloaded.open();
    expect(reloaded.get(record.id)?.state).toBe("prepared");
    expect(reloaded.byKey(record.idempotency_key)?.id).toBe(record.id);
  });

  test("keeps uncertain evidence and expires only settled jobs", () => {
    const data = directory();
    const store = new CreationIntentStore(data);
    const uncertain = { ...intent(data, "unconfirmed"), updatedAt: 1 };
    const settled = { ...intent(data, "settled"), updatedAt: 1 };
    store.insert(uncertain);
    store.insert(settled);

    expect(store.clean(Date.now(), 1_000)).toBe(1);
    expect(store.get(settled.id)).toBeUndefined();
    expect(store.get(uncertain.id)?.state).toBe("unconfirmed");
  });

  test("checkpoints dispatched before the Hub mutation", async () => {
    const data = directory();
    const store = new CreationIntentStore(data);
    const record = intent(data);
    store.insert(record);
    const address = { hub_id: HUB, terminal_id: crypto.randomUUID() };
    let observed = "";
    const ops = {
      async request(method: string) {
        observed = store.get(record.id)?.state ?? "missing";
        if (method === "create") {
          return { creation: { address, creation_capability: crypto.randomUUID() } };
        }
        return {};
      },
    };

    const result = await runIntent(store.get(record.id)!, ops, (value) => store.update(value));
    expect(observed).toBe("dispatched");
    expect(result.state).toBe("settled");
    expect(JSON.stringify(result.result)).not.toContain("creation_capability");
    expect(result.request).toEqual(record.request);
  });
});
