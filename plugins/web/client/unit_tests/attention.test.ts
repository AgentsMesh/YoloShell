import { beforeEach, describe, expect, test } from "bun:test";
import { attentionAgents, needsAgents } from "../browser/attention";
import { dispatch, resetStore } from "../browser/store";
import type { Agent } from "../browser/types";

function agent(
  activity: string,
  interaction = false,
  respond = "unsupported",
  kind = "codex",
  machine = "m1",
): Agent {
  return {
    cursor: {
      target: {
        address: { agent_id: crypto.randomUUID(), machine },
        execution_id: crypto.randomUUID(),
      },
    },
    provider: { kind },
    activity,
    capabilities: { respond_interaction: { support: respond } },
    interaction: interaction ? { reference: { id: crypto.randomUUID() } } : null,
  };
}

function setInventory(inventory: unknown) {
  dispatch({
    type: "catalogue/replace",
    inventory: inventory as any,
    snapshot: undefined,
    emptyStreak: 0,
    at: 0,
  });
}

function hub(provider: string, interaction: "none" | "terminal" | "typed") {
  setInventory({
    hubs: {
      hubs: [
        {
          machine: "m1",
          providers: [{ provider_id: provider, features: { interaction } }],
        },
      ],
    },
  });
}

beforeEach(() => {
  resetStore();
});

describe("attention agents", () => {
  test("includes native approval and turn completion without a typed interaction", () => {
    expect(attentionAgents([agent("waiting_for_approval")])).toHaveLength(1);
    expect(attentionAgents([agent("waiting_for_input")])).toHaveLength(1);
  });

  test("includes an answerable interaction while the agent is working", () => {
    expect(attentionAgents([agent("working", true, "available")])).toHaveLength(1);
  });

  test("excludes working agents without an interaction", () => {
    expect(attentionAgents([agent("working"), agent("unknown")])).toHaveLength(0);
  });
});

describe("needs you", () => {
  test("lists terminal-native approvals from provider features", () => {
    hub("codex", "terminal");
    expect(needsAgents([agent("waiting_for_approval", false, "unsupported", "codex")])).toHaveLength(1);
  });

  test("keeps a typed approval out until the interaction exists", () => {
    hub("claude_code", "typed");
    expect(needsAgents([agent("waiting_for_approval", false, "unsupported", "claude_code")])).toHaveLength(0);
  });

  test("lists a typed approval once it is answerable", () => {
    hub("claude_code", "typed");
    expect(
      needsAgents([agent("waiting_for_approval", true, "available", "claude_code")]),
    ).toHaveLength(1);
  });

  test("falls back to terminal approval for an older Hub without features", () => {
    setInventory({ hubs: { hubs: [{ machine: "m1", providers: [{ provider_id: "codex" }] }] } });
    expect(needsAgents([agent("waiting_for_approval", false, "unsupported", "codex")])).toHaveLength(1);
  });
});
