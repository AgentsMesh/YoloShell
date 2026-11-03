import { describe, expect, test } from "bun:test";
import { HubError } from "../../sdk/src/hubError";
import { InputQueue } from "../../sdk/src/inputQueue";

function leaseLoss() {
  return new HubError(-32000, "invalid writer lease", {
    kind: "hub",
    code: "invalid_lease",
  });
}

class FakeLease {
  state: "observing" | "controlled" | "released" = "controlled";
  currentGrant: object | null = { generation: 1 };
  acquisitions = 0;

  async acquire() {
    this.acquisitions += 1;
    this.state = "controlled";
    this.currentGrant = { generation: 2 };
  }
}

describe("terminal input queue", () => {
  test("retries a lease-rejected chunk once after reacquiring", async () => {
    const lease = new FakeLease();
    const sent: number[][] = [];
    let attempts = 0;
    const input = {
      async input(request: { data: number[] }) {
        attempts += 1;
        if (attempts === 1) throw leaseLoss();
        sent.push(request.data);
      },
    };
    const errors: Error[] = [];
    const queue = new InputQueue(input as any, {} as any, lease as any, (error) => errors.push(error));
    queue.enqueue(Uint8Array.from([1, 2, 3]));
    await Bun.sleep(1);
    expect(lease.acquisitions).toBe(1);
    expect(sent).toEqual([[1, 2, 3]]);
    expect(errors).toEqual([]);
  });

  test("coalesces input that queued behind one delivery into one request", async () => {
    const lease = new FakeLease();
    const calls: number[][] = [];
    let release!: () => void;
    const first = new Promise<void>((resolve) => {
      release = resolve;
    });
    const input = {
      async input(request: { data: number[] }) {
        calls.push(request.data);
        if (calls.length === 1) await first;
      },
    };
    const queue = new InputQueue(input as any, {} as any, lease as any, () => {});
    queue.enqueue(Uint8Array.from([1]));
    queue.enqueue(Uint8Array.from([2]));
    queue.enqueue(Uint8Array.from([3]));
    release();
    await Bun.sleep(1);
    expect(calls).toEqual([[1], [2, 3]]);
  });

  test("does not reacquire after an explicit release", async () => {
    const lease = new FakeLease();
    lease.state = "released";
    lease.currentGrant = null;
    const errors: Error[] = [];
    const queue = new InputQueue(
      { input: async () => {} } as any,
      {} as any,
      lease as any,
      (error) => errors.push(error),
    );
    queue.enqueue(Uint8Array.from([6]));
    await Bun.sleep(1);
    expect(lease.acquisitions).toBe(0);
    expect(errors).toHaveLength(1);
  });

  test("reports and drops the batch when control cannot be recovered", async () => {
    const lease = new FakeLease();
    lease.state = "observing";
    lease.currentGrant = null;
    lease.acquire = async () => {
      throw new Error("held by another client");
    };
    const errors: Error[] = [];
    const queue = new InputQueue(
      { input: async () => {} } as any,
      {} as any,
      lease as any,
      (error) => errors.push(error),
    );
    queue.enqueue(Uint8Array.from([4, 5]));
    await Bun.sleep(1);
    expect(errors).toHaveLength(1);
  });
});
