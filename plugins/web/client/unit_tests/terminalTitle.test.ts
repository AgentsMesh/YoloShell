import { describe, expect, test } from "bun:test";
import { terminalContext, terminalTitle } from "../browser/model";
import { dispatch, resetStore } from "../browser/store";

function setInventory(inventory: unknown) {
  dispatch({
    type: "catalogue/replace",
    inventory: inventory as any,
    snapshot: undefined,
    emptyStreak: 0,
    at: 0,
  });
}

describe("terminal row labels", () => {
  test("prefers a user name when the shell title is only the cwd", () => {
    expect(
      terminalTitle({
        name: "api-work",
        cwd: "/tmp",
        presentation: { title: "stone@host: /tmp" },
      }),
    ).toBe("api-work");
    expect(
      terminalTitle({ name: "api-work", cwd: "/tmp", presentation: { title: "tmp" } }),
    ).toBe("api-work");
  });

  test("keeps a live command title", () => {
    expect(
      terminalTitle({
        name: "bash",
        cwd: "/work",
        presentation: { title: "Respond to greeting" },
      }),
    ).toBe("Respond to greeting");
  });

  test("shows the name next to the path when the title is the name", () => {
    expect(
      terminalContext({ name: "api-work", cwd: "/tmp", presentation: { title: "tmp" } }),
    ).toBe("/tmp");
  });
});

describe("agent row titles", () => {
  test("strips a provider spinner and prefers the explicit terminal name", async () => {
    const { agentTitle } = await import("../browser/model");
    resetStore();
    const terminalId = crypto.randomUUID();
    setInventory({
      terminals: {
        terminals: [
          {
            address: { machine: "m", terminal_id: terminalId },
            name: "bind-agent",
            cwd: "/tmp",
            presentation: { title: "⠴ tmp" },
          },
        ],
      },
      hubs: { hubs: [] },
      agents: { agents: [] },
    });
    expect(
      terminalTitle({ name: "bind-agent", cwd: "/tmp", presentation: { title: "⠴ tmp" } }),
    ).toBe("bind-agent");
    expect(
      agentTitle({
        cursor: { target: { address: { machine: "m", agent_id: "a" } } },
        terminal: { address: { machine: "m", terminal_id: terminalId }, runtime_epoch: "e" },
        provider: { kind: "codex" },
      } as any),
    ).toBe("bind-agent");
  });

  test("prefers an explicit agent name over a prompt-shaped terminal title", async () => {
    const { agentContext, agentTitle } = await import("../browser/model");
    resetStore();
    const terminalId = crypto.randomUUID();
    setInventory({
      terminals: {
        terminals: [
          {
            address: { machine: "m", terminal_id: terminalId },
            name: "ui-agent-4",
            cwd: "/tmp",
            presentation: { title: "Reply with exactly: NEW_AGENT_OK" },
          },
        ],
      },
      hubs: { hubs: [] },
      agents: { agents: [] },
    });
    const agent = {
      cursor: { target: { address: { machine: "m", agent_id: "a" } } },
      terminal: { address: { machine: "m", terminal_id: terminalId }, runtime_epoch: "e" },
      provider: { kind: "codex" },
      working_directory: "/tmp",
    } as any;
    expect(agentTitle(agent)).toBe("ui-agent-4");
    expect(agentContext(agent)).toBe("/tmp");
  });
});
