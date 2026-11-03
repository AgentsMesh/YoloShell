import { describe, expect, test } from "bun:test";
import { TerminalControl } from "../browser/terminalControl";

function harness(autoTake = false) {
  const calls = {
    take: 0,
    release: 0,
    input: [] as number[][],
    stdin: [] as boolean[],
    status: [] as string[],
    error: [] as string[],
    fits: 0,
  };
  const control = new TerminalControl(
    {
      sendTake: () => calls.take++,
      sendRelease: () => calls.release++,
      sendInput: (data) => calls.input.push(Array.from(data)),
      setStdin: (enabled) => calls.stdin.push(enabled),
      setStatus: (text) => calls.status.push(text),
      showTake: () => {},
      showError: (message) => calls.error.push(message),
      clearError: () => {},
      focus: () => {},
      fit: () => calls.fits++,
    },
    autoTake,
  );
  const ready = (state: string) => {
    control.control({ type: "ready" });
    control.control({ type: "lease", state });
  };
  return { control, calls, ready };
}

describe("terminal control", () => {
  test("restored views observe without taking control", () => {
    const { control, calls, ready } = harness(false);
    control.setStream(true, false);
    ready("observing");
    expect(calls.take).toBe(0);
    expect(calls.status.at(-1)).toContain("Read-only");
  });

  test("empty emulator input never counts as control intent", () => {
    const { control, calls, ready } = harness(false);
    control.setStream(true, false);
    ready("observing");
    control.input(new Uint8Array());
    expect(calls.take).toBe(0);
    expect(calls.status.at(-1)).toContain("Read-only");
  });

  test("typing does not steal control from another tab", () => {
    const { control, calls } = harness(false);
    control.setStream(true, false);
    control.control({ type: "ready" });
    control.control({
      type: "lease",
      state: "observing",
      controlled: true,
      controller: false,
      viewers: 2,
    });
    control.input(Uint8Array.from([65]));
    expect(calls.take).toBe(0);
    expect(calls.input).toEqual([]);
    control.requestControl();
    expect(calls.take).toBe(1);
  });

  test("selection observes when another tab already controls", () => {
    const { control, calls } = harness(true);
    control.setStream(true, false);
    control.control({ type: "ready" });
    control.control({
      type: "lease",
      state: "observing",
      controlled: true,
      controller: false,
      viewers: 2,
    });
    expect(calls.take).toBe(0);
    expect(calls.status.at(-1)).toContain("Read-only");
    control.requestControl();
    expect(calls.take).toBe(1);
  });

  test("explicit intent takes control after the lease is known", () => {
    const { control, calls, ready } = harness(false);
    control.requestControl();
    expect(calls.take).toBe(0);
    ready("observing");
    expect(calls.take).toBe(1);
  });

  test("buffers keystrokes until live and flushes them in order", () => {
    const { control, calls, ready } = harness(false);
    ready("controlled");
    control.setStream(false, true);
    control.input(Uint8Array.from([1, 2]));
    control.input(Uint8Array.from([3]));
    expect(calls.input).toEqual([]);
    expect(calls.stdin.at(-1)).toBe(true);
    control.setStream(true, false);
    expect(calls.input).toEqual([[1, 2], [3]]);
  });

  test("a control error drops stale pending input", () => {
    const { control, calls, ready } = harness(false);
    ready("controlled");
    control.setStream(false, true);
    control.input(Uint8Array.from([7]));
    control.control({ type: "error", message: "takeover failed" });
    ready("controlled");
    control.setStream(true, false);
    expect(calls.input).toEqual([]);
    expect(calls.error).toEqual(["takeover failed"]);
  });

  test("typing after a local release retakes before sending bytes", () => {
    const { control, calls, ready } = harness(false);
    control.setStream(true, false);
    ready("controlled");
    control.release();
    control.input(Uint8Array.from([65]));
    expect(calls.release).toBe(1);
    expect(calls.take).toBe(1);
    expect(calls.input).toEqual([]);
    control.control({ type: "lease", state: "controlled" });
    expect(calls.input).toEqual([[65]]);
  });

  test("hiding during takeover cancels intent and releases the lease", () => {
    const { control, calls, ready } = harness(false);
    control.setStream(true, false);
    ready("observing");
    control.requestControl();
    control.input(Uint8Array.from([9]));
    control.release();
    expect(calls.release).toBe(1);
    control.control({ type: "lease", state: "controlled" });
    control.control({ type: "lease", state: "released" });
    expect(calls.input).toEqual([]);
  });

  test("does not fit the emulator while a seed is replaying", () => {
    const { control, calls } = harness(false);
    control.setStream(false, true);
    control.control({ type: "ready" });
    control.control({ type: "lease", state: "controlled" });
    expect(calls.fits).toBe(0);
    control.setStream(true, false);
    expect(calls.fits).toBe(1);
  });
});
