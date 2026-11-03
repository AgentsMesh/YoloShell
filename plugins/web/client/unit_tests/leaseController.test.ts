import { describe, expect, test } from "bun:test";
import { LeaseController } from "../../sdk/src/leaseController";

const target = {
  address: { hub_id: "hub", terminal_id: "terminal" },
  runtime_epoch: "epoch",
};

function grant(generation: number) {
  return {
    generation,
    expires_at_ms: Date.now() + 60_000,
    ttl_ms: 60_000,
    renew_after_ms: 30_000,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => (resolve = done));
  return { promise, resolve };
}

class FakeHub {
  leaseReply = deferred<{ grant: ReturnType<typeof grant> }>();
  renewReply = deferred<{ grant: ReturnType<typeof grant> }>();
  leased = 0;
  renewed = 0;
  released: number[] = [];

  async lease() {
    this.leased += 1;
    return this.leaseReply.promise;
  }

  async renew() {
    this.renewed += 1;
    return this.renewReply.promise;
  }

  async release(grant: { generation: number }) {
    this.released.push(grant.generation);
  }
}

describe("lease controller", () => {
  test("a superseded lease reply never becomes the held grant", async () => {
    const hub = new FakeHub();
    const controller = new LeaseController(hub as any, target, () => {});
    const acquiring = controller.acquire();
    await controller.release();
    hub.leaseReply.resolve({ grant: grant(1) });
    await expect(acquiring).rejects.toThrow("superseded");
    expect(controller.state).toBe("released");
    expect(controller.currentGrant).toBeNull();
    expect(hub.released).toEqual([1]);
  });

  test("a renewal that resolves after release cannot restore a grant", async () => {
    const hub = new FakeHub();
    const controller = new LeaseController(hub as any, target, () => {});
    const acquiring = controller.acquire();
    hub.leaseReply.resolve({ grant: grant(1) });
    await acquiring;
    expect(controller.state).toBe("controlled");
    const serial = (controller as any).renewSerial;
    const renewing = (controller as any).renew(serial);
    await controller.release();
    hub.renewReply.resolve({ grant: grant(2) });
    await renewing;
    expect(controller.state).toBe("released");
    expect(controller.currentGrant).toBeNull();
  });
});
