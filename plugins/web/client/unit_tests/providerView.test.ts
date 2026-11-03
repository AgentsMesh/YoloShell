import { describe, expect, test } from "bun:test";
import { providerOptions } from "../server/providerView";

describe("server provider view", () => {
  test("sorts and validates advertised providers", () => {
    expect(
      providerOptions([
        { provider_id: "gemini", display_name: "Gemini", available: false },
        { provider_id: "codex", display_name: "Codex", available: true },
      ]),
    ).toEqual([
      { provider_id: "codex", display_name: "Codex", available: true },
      { provider_id: "gemini", display_name: "Gemini", available: false },
    ]);
  });

  test("drops malformed, duplicate and unsafe rows", () => {
    expect(
      providerOptions([
        { provider_id: "codex", display_name: "Codex", available: true },
        { provider_id: "codex", display_name: "Other", available: false },
        { provider_id: "bad/../id", display_name: "Bad", available: true },
        { provider_id: "qwen_code", display_name: "bad\nname", available: true },
        { provider_id: "gemini", display_name: "Gemini" },
        null,
      ]),
    ).toEqual([{ provider_id: "codex", display_name: "Codex", available: true }]);
  });

  test("forwards validated provider features and drops malformed ones", () => {
    expect(
      providerOptions([
        {
          provider_id: "codex",
          display_name: "Codex",
          available: true,
          features: {
            conversation: true,
            submit_prompt: true,
            interrupt: true,
            interaction: "terminal",
          },
        },
        {
          provider_id: "gemini",
          display_name: "Gemini",
          available: true,
          features: { conversation: true },
        },
      ]),
    ).toEqual([
      {
        provider_id: "codex",
        display_name: "Codex",
        available: true,
        features: {
          conversation: true,
          submit_prompt: true,
          interrupt: true,
          interaction: "terminal",
        },
      },
      { provider_id: "gemini", display_name: "Gemini", available: true },
    ]);
  });

  test("a non-list is empty, never an improvised catalogue", () => {
    expect(providerOptions(undefined)).toEqual([]);
    expect(providerOptions({ providers: [] })).toEqual([]);
  });
});
