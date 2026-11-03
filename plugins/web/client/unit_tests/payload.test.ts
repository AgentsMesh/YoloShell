import { describe, expect, test } from "bun:test";
import { canSend } from "../browser/payload";
import type { Selected } from "../browser/types";

function agent(overrides: Record<string, unknown> = {}): Selected {
  return {
    kind: "agent",
    agent: {
      lifecycle: { state: "running" },
      reachability: "reachable",
      freshness: "current",
      capabilities: { submit_prompt: { support: "available" } },
      ...overrides,
    },
  };
}

describe("prompt admission", () => {
  test("follows the observed submit capability, not the snapshot schema", () => {
    expect(canSend(agent())).toBe(true);
    expect(canSend(agent({ schema_version: 3 }))).toBe(true);
  });

  test("rejects stale, unreachable, ended or incapable agents", () => {
    expect(canSend(agent({ freshness: "stale" }))).toBe(false);
    expect(canSend(agent({ reachability: "unreachable" }))).toBe(false);
    expect(canSend(agent({ lifecycle: { state: "exited" } }))).toBe(false);
    expect(canSend(agent({ capabilities: { submit_prompt: { support: "unsupported" } } }))).toBe(
      false,
    );
  });

  test("requires a running runtime for a shell", () => {
    expect(
      canSend({
        kind: "terminal",
        terminal: { runtime: { status: "running" } },
      }),
    ).toBe(true);
    expect(
      canSend({
        kind: "terminal",
        terminal: { runtime: { status: "exited" } },
      }),
    ).toBe(false);
  });
});
