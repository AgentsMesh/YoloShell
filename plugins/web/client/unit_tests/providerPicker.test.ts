import { describe, expect, test } from "bun:test";
import {
  providerAvailable,
  providerChoices,
  providerSupported,
} from "../browser/providerPicker";

const HUB = {
  providers: [
    { provider_id: "codex", display_name: "Codex", available: true },
    { provider_id: "claude_code", display_name: "Claude Code", available: false },
  ],
};

describe("web provider picker", () => {
  test("uses an advertised per-machine list and keeps unavailable providers visible", () => {
    expect(providerChoices(HUB)).toEqual([
      { id: "codex", label: "Codex", available: true },
      { id: "claude_code", label: "Claude Code", available: false },
    ]);
    expect(providerSupported(HUB)).toBe(true);
    expect(providerAvailable(HUB, "codex")).toBe(true);
    expect(providerAvailable(HUB, "claude_code")).toBe(false);
  });

  test("falls back to the static list when an older Hub advertises nothing", () => {
    expect(providerChoices({})).toEqual([]);
    expect(providerSupported({})).toBe(true);
    expect(providerAvailable({}, "claude_code")).toBe(true);
    expect(providerChoices({ providers: [] })).toEqual([]);
  });

  test("a machine with no available provider cannot start an agent", () => {
    const unavailable = {
      providers: [
        { provider_id: "codex", display_name: "Codex", available: false },
        { provider_id: "gemini", display_name: "Gemini", available: false },
      ],
    };
    expect(providerSupported(unavailable)).toBe(false);
    expect(providerAvailable(unavailable, "codex")).toBe(false);
  });

  test("forwards validated provider features and drops malformed ones", () => {
    const hub = {
      providers: [
        {
          provider_id: "claude_code",
          display_name: "Claude Code",
          available: true,
          features: {
            conversation: true,
            submit_prompt: true,
            interrupt: true,
            interaction: "typed",
          },
        },
        {
          provider_id: "codex",
          display_name: "Codex",
          available: true,
          features: { conversation: true },
        },
      ],
    };
    expect(providerChoices(hub)).toEqual([
      {
        id: "claude_code",
        label: "Claude Code",
        available: true,
        features: {
          conversation: true,
          submitPrompt: true,
          interrupt: true,
          interaction: "typed",
        },
      },
      { id: "codex", label: "Codex", available: true },
    ]);
  });

  test("malformed provider rows are ignored rather than guessed", () => {
    const malformed = {
      providers: [
        { provider_id: "", display_name: "Empty", available: true },
        { provider_id: "codex", display_name: "", available: true },
        { provider_id: "codex", display_name: "Codex", available: "yes" },
        null,
      ],
    };
    expect(providerChoices(malformed as any)).toEqual([]);
  });
});
