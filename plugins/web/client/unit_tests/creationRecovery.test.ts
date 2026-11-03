import { describe, expect, test } from "bun:test";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { HubTimeoutError } from "../../sdk/src/hubWire";
import { CreationCoordinator } from "../server/creationJobs";
import { CreationIntentStore } from "../server/creationIntent";
import { prepare, spec } from "../server/creationRequest";
import { ADDRESS, TIMING, body, fake, key, store, until } from "./creationFixture";

describe("creation recovery", () => {
  test("never replays Create when QueryCreation reports missing", async () => {
    let createCalls = 0;
    let queries = 0;
    const { ops } = fake((method, params) => {
      if (method === "create") {
        createCalls += 1;
        throw new HubTimeoutError("lost reply");
      }
      if (method === "query_creation") {
        queries += 1;
        return {
          query: {
            creation_token: params.request.creation_token,
            outcome: { status: "missing" },
          },
        };
      }
      throw new Error(`unexpected ${method}`);
    });
    const data = store();
    const coordinator = new CreationCoordinator(data, ops, TIMING);
    const job = coordinator.start("shell", { ...body(), idempotency_key: key() }) as any;
    await until(() => data.get(job.job_id)?.attempts === 2);
    expect(createCalls).toBe(1);
    expect(queries).toBe(1);
    expect((coordinator.job(job.job_id) as any).state).toBe("unknown");
  });

  test("reuses the agent start token when a reply is lost", async () => {
    let attempts = 0;
    const { ops, calls } = fake((method) => {
      if (method !== "agent_start") throw new Error(`unexpected ${method}`);
      attempts += 1;
      if (attempts === 1) throw new HubTimeoutError("lost reply");
      return { start: { address: ADDRESS, provider_id: "codex", binding: "awaiting" } };
    });
    const coordinator = new CreationCoordinator(store(), ops, TIMING);
    const job = coordinator.start("agent", {
      ...body(),
      provider: "codex",
      idempotency_key: key(),
    }) as any;
    await until(() => (coordinator.job(job.job_id) as any).state === "done");
    expect(calls).toHaveLength(2);
    expect(calls[0].params.request.start_token).toBe(calls[1].params.request.start_token);
    expect((coordinator.job(job.job_id) as any).result.binding).toBe("awaiting");
  });

  test("recovers a restart-dispatched shell from durable evidence", async () => {
    const directory = mkdtempSync(join(tmpdir(), "yolo-web-restart-"));
    const first = new CreationIntentStore(directory);
    const prepared = prepare(spec("shell", body()));
    const now = Date.now();
    first.insert({
      version: 1,
      id: crypto.randomUUID(),
      idempotency_key: key(),
      fingerprint: "fingerprint",
      kind: "shell",
      origin: prepared.origin,
      request: prepared.request,
      state: "dispatched",
      attempts: 0,
      createdAt: now,
      updatedAt: now,
    });
    const { ops, calls } = fake((method, params) => {
      if (method === "query_creation") {
        return {
          query: {
            creation_token: params.request.creation_token,
            outcome: { status: "finalized", creation: { address: ADDRESS } },
          },
        };
      }
      throw new Error(`unexpected ${method}`);
    });
    const second = new CreationIntentStore(directory);
    const coordinator = new CreationCoordinator(second, ops, TIMING);
    coordinator.recover();
    const record = second.all()[0];
    await until(() => (coordinator.job(record.id) as any).state === "done");
    expect(calls[0].method).toBe("query_creation");
    expect(calls[0].params.request).toEqual(prepared.request);
    expect((coordinator.job(record.id) as any).result.terminal.terminal_id).toBe(
      ADDRESS.terminal_id,
    );
  });
});
