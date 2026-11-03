import { describe, expect, test } from "bun:test";
import { HubError } from "../../sdk/src/index";
import { TerminalLease } from "../server/terminalLease";

const target = {
  address: { hub_id: "hub", terminal_id: "terminal" },
  runtime_epoch: "epoch",
};

type FakeSession = {
  state: string;
  connected: boolean;
  generation: number | null;
  observeError?: Error;
  closes: number;
  observed: number;
  acquired: number;
  taken: number[];
};

function fake(overrides: Partial<FakeSession> = {}): FakeSession & Record<string, any> {
  const session: FakeSession & Record<string, any> = {
    state: "observing",
    connected: true,
    generation: null,
    closes: 0,
    observed: 0,
    acquired: 0,
    taken: [],
    ...overrides,
    async observe() {
      session.observed += 1;
      if (session.observeError) throw session.observeError;
      return session.generation;
    },
    async acquire() {
      session.acquired += 1;
      session.state = "controlled";
    },
    async take(generation: number) {
      session.taken.push(generation);
      session.state = "controlled";
    },
    async close() {
      session.closes += 1;
    },
    enqueue() {},
    resize: async () => {},
    abandon() {},
  };
  return session;
}

function lease(sessions: FakeSession[]) {
  let index = 0;
  const opened: FakeSession[] = [];
  const value = new TerminalLease(
    target,
    () => {},
    () => {},
    async () => {
      const session = sessions[Math.min(index++, sessions.length - 1)];
      opened.push(session);
      return session as any;
    },
  );
  return { value, opened };
}

describe("terminal lease session", () => {
  test("a dead Hub session is replaced before taking control", async () => {
    const dead = fake({ state: "controlled", connected: false });
    const fresh = fake({ generation: 7 });
    const { value, opened } = lease([dead, fresh]);

    expect(await value.take()).toBe(true);
    expect(opened).toEqual([dead, fresh]);
    expect(dead.closes).toBe(1);
    expect(fresh.taken).toEqual([7]);
  });

  test("a transport failure rebuilds the private Hub sockets once", async () => {
    const broken = fake({ observeError: new Error("Hub is not connected") });
    const fresh = fake();
    const { value, opened } = lease([broken, fresh]);

    expect(await value.take()).toBe(true);
    expect(opened).toEqual([broken, fresh]);
    expect(broken.closes).toBe(1);
    expect(fresh.acquired).toBe(1);
  });

  test("a reachable Hub conflict is not retried on a new session", async () => {
    const conflict = new HubError(-32000, "operation conflicts", { code: "conflict" });
    const broken = fake({ observeError: conflict });
    const fresh = fake();
    const { value, opened } = lease([broken, fresh]);

    await expect(value.take()).rejects.toBeInstanceOf(HubError);
    expect(opened).toEqual([broken]);
    expect(broken.closes).toBe(0);
  });
});
