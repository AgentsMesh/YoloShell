import { describe, expect, test } from "bun:test";
import {
  application,
  coherent,
  finishedTerminal,
  liveAgent,
  liveTerminal,
  selectableTerminal,
  visibleAgent,
} from "../browser/catalogue";
import { catalogue } from "../server/inventory";

const EMPTY = {
  terminals: { terminals: [] },
  agents: { agents: [] },
  hubs: { hubs: [] },
};
const ONE = {
  terminals: { terminals: [{ address: { terminal_id: "t" }, runtime: { status: "running" } }] },
  agents: { agents: [{ cursor: { target: { address: { agent_id: "a" } } }, lifecycle: { state: "running" } }] },
  hubs: { hubs: [] },
};
const SNAPSHOT = { server: "s", sequence: 1 };

describe("catalogue visibility", () => {
  test("live terminals include starting, blocked output and finalizing", () => {
    for (const status of ["starting", "running", "output_blocked", "finalizing"]) {
      expect(liveTerminal({ runtime: { status } }), status).toBe(true);
    }
    for (const status of ["exited", "lost", "unreachable", undefined]) {
      expect(liveTerminal({ runtime: { status } }), String(status)).toBe(false);
    }
  });

  test("finished sessions remain selectable but are not live", () => {
    for (const status of ["exited", "lost"]) {
      const terminal = { runtime: { status } };
      expect(finishedTerminal(terminal), status).toBe(true);
      expect(liveTerminal(terminal), status).toBe(false);
      expect(selectableTerminal(terminal), status).toBe(true);
    }
    expect(finishedTerminal({ runtime: { status: "running" } })).toBe(false);
    expect(selectableTerminal({ runtime: { status: "unreachable" } })).toBe(false);
  });

  test("live agents include a starting turn", () => {
    expect(liveAgent({ lifecycle: { state: "starting" } })).toBe(true);
    expect(liveAgent({ lifecycle: { state: "running" } })).toBe(true);
    expect(liveAgent({ lifecycle: { state: "unknown" } })).toBe(false);
    expect(liveAgent({ lifecycle: { state: "exited" } })).toBe(false);
  });

  test("a live terminal keeps its agent visible across a degraded observation", () => {
    const terminal = { address: { terminal_id: "t" }, runtime: { status: "running" } };
    const agent = (state: string, reachability: string, freshness: string) => ({
      lifecycle: { state },
      reachability,
      freshness,
      terminal: { address: { terminal_id: "t" } },
    });
    expect(visibleAgent(agent("running", "unreachable", "stale"), [terminal])).toBe(true);
    expect(visibleAgent(agent("unknown", "reachable", "current"), [terminal])).toBe(true);
    expect(visibleAgent(agent("unknown", "unreachable", "stale"), [terminal])).toBe(false);
    expect(visibleAgent(agent("running", "reachable", "current"), [
      { ...terminal, runtime: { status: "exited" } },
    ])).toBe(false);
    expect(visibleAgent(agent("running", "unreachable", "stale"), [])).toBe(false);
    const omitted = { ...agent("running", "reachable", "current"), terminal: { address: { terminal_id: "other" } } };
    expect(visibleAgent(omitted, [])).toBe(true);
  });

  test("a reply without every catalogue section is incoherent", () => {
    expect(coherent(ONE)).toBe(true);
    expect(coherent({ ...ONE, agents: {} })).toBe(false);
    expect(coherent({ ...ONE, hubs: undefined })).toBe(false);
    expect(coherent(null)).toBe(false);
  });
});

describe("catalogue application", () => {
  test("an out-of-order snapshot never rolls the catalogue backwards", () => {
    const applied = { server: "s", sequence: 4 };
    expect(application(ONE, applied, { ...ONE, snapshot: { server: "s", sequence: 5 } }, 0).apply).toBe(true);
    expect(application(ONE, applied, { ...ONE, snapshot: { server: "s", sequence: 4 } }, 0).apply).toBe(false);
    expect(application(ONE, applied, { ...ONE, snapshot: { server: "s", sequence: 3 } }, 0).apply).toBe(false);
    expect(application(ONE, applied, { ...ONE, snapshot: { server: "other", sequence: 1 } }, 0).apply).toBe(true);
    expect(application(ONE, applied, { ...ONE, snapshot: undefined }, 0).apply).toBe(true);
  });

  test("one empty reply after a populated catalogue is not rendered", () => {
    const first = application(ONE, SNAPSHOT, { ...EMPTY, snapshot: { server: "s", sequence: 2 } }, 0);
    expect(first).toEqual({ apply: false, emptyStreak: 1 });
    const second = application(ONE, SNAPSHOT, { ...EMPTY, snapshot: { server: "s", sequence: 3 } }, first.emptyStreak);
    expect(second.apply).toBe(true);
  });

  test("a non-empty reply resets the confirmation streak", () => {
    expect(application(ONE, SNAPSHOT, { ...ONE, snapshot: { server: "s", sequence: 2 } }, 1)).toEqual({
      apply: true,
      emptyStreak: 0,
    });
  });
});

describe("server catalogue", () => {
  test("passes every row through without a serviceability filter", () => {
    const raw = {
      terminals: { terminals: [{ runtime: { status: "output_blocked" } }, { runtime: { status: "exited" } }] },
      agents: { agents: [{ lifecycle: { state: "unknown" } }] },
      hubs: { hubs: [{ reachable: true }] },
    };
    expect(catalogue(raw)).toEqual({
      terminals: raw.terminals.terminals,
      agents: raw.agents.agents,
      hubs: raw.hubs.hubs,
    });
  });

  test("a missing catalogue section is an error, not an empty list", () => {
    expect(() => catalogue({ terminals: {}, agents: {}, hubs: {} })).toThrow();
    expect(() => catalogue({ terminals: { terminals: [] }, agents: {}, hubs: { hubs: [] } })).toThrow();
    expect(() => catalogue(null)).toThrow();
  });
});
