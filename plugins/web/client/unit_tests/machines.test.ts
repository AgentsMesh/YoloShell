import { describe, expect, test } from "bun:test";
import { machineLabel } from "../browser/machines";
import { inScope } from "../browser/appSelectors";
import { initialState } from "../browser/appState";

describe("machine picker", () => {
  test("prefers a configured label, then names the entry Hub", () => {
    const entry = crypto.randomUUID();
    const peer = crypto.randomUUID();
    expect(machineLabel({ machine: peer, label: "build-box" }, entry)).toBe("build-box");
    expect(machineLabel({ machine: entry }, entry)).toBe("This machine");
    expect(machineLabel({ machine: peer }, entry)).toBe("Machine");
  });

  test("scopes rows only when a machine is selected", () => {
    const peer = crypto.randomUUID();
    const state = initialState({ quiet: false, notify: false, pendingAgents: [] });
    expect(inScope(peer, state)).toBe(true);
    state.ui.scope = peer;
    expect(inScope(peer, state)).toBe(true);
    expect(inScope(crypto.randomUUID(), state)).toBe(false);
  });
});
