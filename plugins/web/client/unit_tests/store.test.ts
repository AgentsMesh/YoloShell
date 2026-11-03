import { beforeEach, describe, expect, test } from "bun:test";
import { selected, selectedForIntent } from "../browser/appSelectors";
import { emptyConversation } from "../browser/appState";
import { entryKey } from "../browser/conversationModel";
import { dispatch, getState, resetStore } from "../browser/store";
import type { Agent, Entry, Terminal } from "../browser/types";

const entry = (text: string): Entry => ({
  id: [text.charCodeAt(0)],
  role: "user",
  timestamp: null,
  parts: [{ kind: "text", text }],
});

function terminal(id: string, status = "running"): Terminal {
  return { address: { machine: "hub-a", terminal_id: id }, runtime: { status } };
}

function agent(id: string, terminalId: string): Agent {
  return {
    cursor: {
      target: { address: { machine: "hub-a", agent_id: id }, execution_id: `exec-${id}` },
    },
    terminal: {
      address: { machine: "hub-a", terminal_id: terminalId },
      runtime_epoch: "epoch",
    },
    lifecycle: { state: "running" },
    reachability: "reachable",
    freshness: "current",
    provider: { kind: "codex" },
  };
}

function setInventory(terminals: Terminal[], agents: Agent[] = []): void {
  dispatch({
    type: "catalogue/replace",
    inventory: {
      machine: "hub-a",
      terminals: { terminals },
      agents: { agents },
      hubs: { hubs: [{ machine: "hub-a", reachable: true }] },
    },
    snapshot: undefined,
    emptyStreak: 0,
    at: 1,
  });
}

beforeEach(() => {
  resetStore();
});

describe("application store", () => {
  test("choosing a terminal adopts its lens and resets conversation state", () => {
    setInventory([terminal("t1")]);
    dispatch({
      type: "conversation/latest",
      entries: [entry("old")],
      nextCursor: { before: 2 },
      partial: ["unsupported_record"],
    });
    dispatch({ type: "request/status", text: "stale", kind: "bad" });
    const before = getState().selection.epoch;

    dispatch({
      type: "selection/choose",
      intent: { kind: "terminal", id: "t1", machine: "hub-a", take: true },
    });
    const state = getState();
    expect(state.selection.selected).toEqual({
      kind: "terminal",
      id: "t1",
      machine: "hub-a",
    });
    expect(selected(state)).toEqual({ kind: "terminal", terminal: terminal("t1") });
    expect(state.selection.lens).toBe("terminal");
    expect(state.selection.epoch).toBe(before + 1);
    expect(state.conversation).toEqual(emptyConversation());
    expect(state.request.status).toEqual({ text: "", kind: "" });
  });

  test("refreshes a selected row in place when the same identity survives", () => {
    setInventory([terminal("t1")]);
    dispatch({
      type: "selection/choose",
      intent: { kind: "terminal", id: "t1", machine: "hub-a", take: false },
    });
    dispatch({
      type: "conversation/latest",
      entries: [entry("kept")],
      nextCursor: null,
      partial: [],
    });
    setInventory([terminal("t1")]);
    const state = getState();
    expect(state.selection.selected?.kind).toBe("terminal");
    expect(selected(state)?.kind).toBe("terminal");
    expect(state.conversation.entries).toHaveLength(1);
  });

  test("clears a stale selection and promotes a shell claimed by an agent", () => {
    setInventory([terminal("t1")]);
    dispatch({
      type: "selection/choose",
      intent: { kind: "terminal", id: "t1", machine: "hub-a", take: false },
    });
    setInventory([terminal("t1")], [agent("a1", "t1")]);
    const promoted = getState().selection.selected;
    expect(promoted).toEqual({ kind: "agent", id: "a1", machine: "hub-a" });
    const resolved = selected(getState());
    expect(resolved?.kind).toBe("agent");
    expect(
      resolved?.kind === "agent" &&
        resolved.agent.cursor?.target?.address?.agent_id,
    ).toBe("a1");

    setInventory([]);
    expect(getState().selection.selected).toBeNull();
  });

  test("deduplicates older pages and keeps the partial note explicit", () => {
    dispatch({
      type: "conversation/latest",
      entries: [entry("b"), entry("c")],
      nextCursor: { before: 20 },
      partial: [],
    });
    dispatch({
      type: "conversation/olderAppend",
      entries: [entry("a"), entry("b")],
      nextCursor: null,
      partial: ["unsupported_record"],
    });
    const state = getState().conversation;
    expect(state.entries.map(entryKey)).toEqual(
      [entry("a"), entry("b"), entry("c")].map(entryKey),
    );
    expect(state.nextCursor).toBeNull();
    expect(state.note).toBe("Some history is unavailable");
  });

  test("observing a skipped empty reply keeps the catalogue", () => {
    setInventory([terminal("t1")]);
    const inventory = getState().catalogue.inventory;
    dispatch({ type: "catalogue/observe", emptyStreak: 2, at: 99 });
    const state = getState();
    expect(state.catalogue.inventory).toBe(inventory);
    expect(state.catalogue.connected).toBe(true);
    expect(state.catalogue.updatedAt).toBe(99);
    expect(state.catalogue.emptyStreak).toBe(2);
  });

  test("a scope change clears a selection outside the machine", () => {
    setInventory([terminal("t1")]);
    dispatch({
      type: "selection/choose",
      intent: { kind: "terminal", id: "t1", machine: "hub-a", take: false },
    });
    dispatch({ type: "ui/scope", scope: "hub-b" });
    expect(getState().selection.selected).toBeNull();
  });

  test("pending selection is resolved only when the row becomes selectable", () => {
    const intent = { kind: "terminal" as const, id: "t1", machine: "hub-a", take: true };
    dispatch({ type: "selection/created", intent });
    expect(getState().selection.pending).toEqual(intent);
    expect(selectedForIntent(getState(), intent)).toBeNull();
    setInventory([terminal("t1", "starting")]);
    expect(selectedForIntent(getState(), intent)).not.toBeNull();
  });
});
