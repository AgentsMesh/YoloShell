import { describe, expect, test } from "bun:test";
import { HubTimeoutError } from "../../sdk/src/hubWire";
import { CreationCoordinator } from "../server/creationJobs";
import {
  ADDRESS,
  TIMING,
  body,
  created,
  fake,
  key,
  store,
  until,
} from "./creationFixture";

describe("durable creation jobs", () => {
  test("deduplicates a retry key and journals before dispatch", async () => {
    const data = store();
    let observed = "";
    const { ops, calls } = created();
    const original = ops.request;
    ops.request = async (method: string, params: any) => {
      if (method === "create") observed = data.all()[0]?.state ?? "missing";
      return original(method, params);
    };
    const coordinator = new CreationCoordinator(data, ops, TIMING);
    const idempotency_key = key();
    const first = coordinator.start("shell", { ...body(), idempotency_key });
    const second = coordinator.start("shell", { ...body(), idempotency_key });
    const trimmed = coordinator.start("shell", {
      ...body(),
      cwd: " /tmp ",
      idempotency_key,
    });
    expect(second.job_id).toBe(first.job_id);
    expect(trimmed.job_id).toBe(first.job_id);
    expect(second.state).toBe("running");
    await until(() => (coordinator.job(first.job_id) as any).state === "done");
    expect(calls.filter((call) => call.method === "create")).toHaveLength(1);
    expect(observed).toBe("dispatched");
  });

  test("rejects one retry key used for a different request", () => {
    const { ops } = created();
    const coordinator = new CreationCoordinator(store(), ops, TIMING);
    const idempotency_key = key();
    coordinator.start("shell", { ...body(), name: "one", idempotency_key });
    expect(() =>
      coordinator.start("shell", { ...body(), name: "two", idempotency_key }),
    ).toThrow("different creation request");
  });

  test("recovers a lost shell reply through QueryCreation and finalize", async () => {
    const capability = crypto.randomUUID();
    let createCalls = 0;
    const { ops, calls } = fake((method, params) => {
      if (method === "create") {
        createCalls += 1;
        throw new HubTimeoutError("lost reply");
      }
      if (method === "query_creation") {
        return {
          query: {
            creation_token: params.request.creation_token,
            outcome: {
              status: "ready",
              creation: { address: ADDRESS, creation_capability: capability },
            },
          },
        };
      }
      if (method === "finalize") return {};
      throw new Error(`unexpected ${method}`);
    });
    const coordinator = new CreationCoordinator(store(), ops, TIMING);
    const job = coordinator.start("shell", { ...body(), idempotency_key: key() }) as any;
    await until(() => (coordinator.job(job.job_id) as any).state === "done");
    expect(createCalls).toBe(1);
    expect(calls.map((call) => call.method)).toEqual(["create", "query_creation", "finalize"]);
    const result = (coordinator.job(job.job_id) as any).result;
    expect(result.terminal.terminal_id).toBe(ADDRESS.terminal_id);
    expect(JSON.stringify(result)).not.toContain(capability);
  });
});
