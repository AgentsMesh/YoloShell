import { beforeEach, describe, expect, test } from "bun:test";
import { selected, selectedForIntent } from "../browser/appSelectors";
import { dispatch, getState, resetStore } from "../browser/store";
import type { SelectionIntent, Terminal } from "../browser/types";

function terminal(id: string, status = "running"): Terminal {
  return { address: { machine: "hub-a", terminal_id: id }, runtime: { status } };
}

function setInventory(terminals: Terminal[]): void {
  dispatch({
    type: "catalogue/replace",
    inventory: {
      machine: "hub-a",
      terminals: { terminals },
      agents: { agents: [] },
      hubs: { hubs: [{ machine: "hub-a", reachable: true }] },
    },
    snapshot: undefined,
    emptyStreak: 0,
    at: 0,
  });
}

const intent = (id: string): SelectionIntent => ({
  kind: "terminal",
  id,
  machine: "hub-a",
  take: true,
});

beforeEach(() => {
  resetStore();
});

describe("selection intent", () => {
  test("holds a created terminal until the catalogue reports it", () => {
    dispatch({ type: "selection/created", intent: intent("t1") });
    expect(getState().selection.selected).toBeNull();
    expect(selectedForIntent(getState(), intent("t1"))).toBeNull();
    expect(getState().selection.pending?.id).toBe("t1");

    setInventory([terminal("t1")]);
    expect(selectedForIntent(getState(), intent("t1"))).toEqual({
      kind: "terminal",
      terminal: terminal("t1"),
    });
    dispatch({ type: "selection/adopt", intent: intent("t1") });
    expect(getState().selection.pending).toBeNull();
    expect(getState().selection.selected).toEqual({
      kind: "terminal",
      id: "t1",
      machine: "hub-a",
    });
    expect(selected(getState())).toEqual({
      kind: "terminal",
      terminal: terminal("t1"),
    });
  });

  test("does not adopt a terminal reported without a live state", () => {
    dispatch({ type: "selection/created", intent: intent("t1") });
    setInventory([terminal("t1", "")]);
    expect(selectedForIntent(getState(), intent("t1"))).toBeNull();
    expect(getState().selection.pending?.id).toBe("t1");
  });

  test("holding a selection replaces the previous selection", () => {
    setInventory([terminal("old")]);
    dispatch({
      type: "selection/choose",
      intent: { kind: "terminal", id: "old", machine: "hub-a", take: false },
    });
    expect(getState().selection.selected).not.toBeNull();
    dispatch({ type: "selection/created", intent: intent("t1") });
    expect(getState().selection.selected).toBeNull();
    expect(getState().selection.pending?.id).toBe("t1");
  });
});
