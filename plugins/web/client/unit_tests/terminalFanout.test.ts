import { describe, expect, test } from "bun:test";
import { TerminalControlFanout } from "../server/terminalControlFanout";
import { TerminalOutput } from "../server/terminalOutput";
import type { Seed } from "../server/terminalSeed";

function socket() {
  const frames: any[] = [];
  return {
    frames,
    readyState: 1,
    send: (value: any) => frames.push(value),
    close: () => undefined,
    getBufferedAmount: () => 0,
  } as any;
}

function leases(frames: any[]): any[] {
  return frames
    .filter((frame) => typeof frame === "string")
    .map((frame) => JSON.parse(frame))
    .filter((frame) => frame.type === "lease");
}

describe("terminal control fan-out", () => {
  test("one actor lease is projected as controlled only to the controller", () => {
    const actor = new TerminalControlFanout();
    const first = socket();
    const second = socket();
    actor.add(first, "first");
    actor.add(second, "second");
    actor.acquire("first");
    expect(actor.owns("first")).toBe(true);
    expect(actor.owns("second")).toBe(false);
    actor.assign("first");
    expect(leases(first.frames).at(-1)).toMatchObject({
      state: "controlled",
      controller: true,
      viewers: 2,
    });
    expect(leases(second.frames).at(-1)).toMatchObject({
      state: "observing",
      controller: false,
      controlled: true,
    });
  });

  test("a local takeover demotes the old controller without a Hub call", () => {
    const actor = new TerminalControlFanout();
    const first = socket();
    const second = socket();
    actor.add(first, "first");
    actor.add(second, "second");
    actor.assign("first");
    actor.assign("second");
    expect(leases(first.frames).at(-1)).toMatchObject({ state: "observing", controlled: true });
    expect(leases(second.frames).at(-1)).toMatchObject({ state: "controlled", controller: true });
    expect(actor.remove(second, "second")).toBe(true);
    expect(leases(first.frames).at(-1)).toMatchObject({ state: "observing", controlled: false });
  });
});

describe("terminal output fan-out", () => {
  const seed: Seed = {
    kind: "snapshot",
    cols: 80,
    rows: 24,
    data: Uint8Array.from([1, 2, 3]),
  };

  test("a late subscriber receives the shared seed plus bytes since the seed", () => {
    const output = new TerminalOutput();
    output.install(seed);
    const first = socket();
    output.subscribe(first);
    expect(first.frames[0]).toEqual(expect.stringContaining('"type":"ready"'));
    output.publish(Uint8Array.from([4, 5]));
    const late = socket();
    output.subscribe(late);
    expect(late.frames[0]).toEqual(expect.stringContaining('"type":"ready"'));
    expect(late.frames[1]).toEqual(Uint8Array.from([1, 2, 3]));
    expect(late.frames[2]).toEqual(Uint8Array.from([4, 5]));
    expect(late.frames[3]).toEqual(expect.stringContaining('"type":"live"'));
  });

  test("a late subscriber receives the controller's current size", () => {
    const output = new TerminalOutput();
    output.install(seed);
    output.resize(120, 40);
    const late = socket();
    output.subscribe(late);
    expect(JSON.parse(late.frames[0])).toMatchObject({
      type: "ready",
      cols: 120,
      rows: 40,
    });
  });

  test("ring overflow asks the actor for a fresh seed before a late subscriber", () => {
    const output = new TerminalOutput();
    output.install(seed);
    const first = socket();
    output.subscribe(first);
    output.publish(new Uint8Array(9 * 1024 * 1024));
    expect(output.needsSeed).toBe(true);
    const late = socket();
    output.subscribe(late);
    expect(late.frames).toEqual([]);
  });
});
