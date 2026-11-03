import { describe, expect, test } from "bun:test";
import {
  ready,
  lease,
  release,
  requestControl,
} from "../browser/terminalControlLease";
import {
  initial,
  presentation,
  reset,
  setStream,
} from "../browser/terminalControlState";

describe("terminal control machine", () => {
  test("moves through connecting, seeding and live stream phases", () => {
    let machine = initial(false);
    expect(presentation(machine).status).toContain("Connecting");
    machine = setStream(machine, false, true).machine;
    expect(presentation(machine).status).toContain("Loading");
    machine = setStream(machine, true, false).machine;
    expect(presentation(machine).status).toContain("Read-only");
    machine = reset(machine).machine;
    expect(machine.stream).toBe("connecting");
    expect(machine.lease).toBe("unknown");
  });

  test("takes control only after ready and a known lease", () => {
    let machine = initial(false);
    machine = setStream(machine, true, false).machine;
    let step = requestControl(machine);
    expect(step.effects).toContainEqual({ kind: "render" });
    expect(step.effects).toContainEqual({ kind: "clearError" });
    machine = ready(step.machine).machine;
    step = lease(machine, { type: "lease", state: "observing" });
    expect(step.effects.some((effect) => effect.kind === "take")).toBe(true);
    expect(step.machine.takeSent).toBe(true);
  });

  test("a controlled lease flushes buffered input and fits once live", () => {
    let machine = initial(false);
    machine = setStream(machine, true, false).machine;
    machine = ready(machine).machine;
    machine = lease(machine, { state: "observing" }).machine;
    machine = requestControl(machine).machine;
    const buffered = {
      ...machine,
      pending: [Uint8Array.from([1]), Uint8Array.from([2])],
      bytes: 2,
    };
    const step = lease(buffered, {
      state: "controlled",
      controlled: true,
      controller: true,
      viewers: 1,
    });
    expect(step.effects).toContainEqual({ kind: "input", data: Uint8Array.from([1]) });
    expect(step.effects).toContainEqual({ kind: "input", data: Uint8Array.from([2]) });
    expect(step.effects).toContainEqual({ kind: "fit" });
    expect(step.machine.pending).toEqual([]);
  });

  test("release publishes an explicit released lease", () => {
    let machine = initial(false);
    machine = setStream(machine, true, false).machine;
    machine = lease(machine, { state: "controlled", controlled: true, controller: true }).machine;
    const step = release(machine);
    expect(step.machine.lease).toBe("released");
    expect(step.effects).toContainEqual({ kind: "release" });
  });
});
