import { describe, expect, test } from "bun:test";
import {
  WEB_PROTOCOL,
  conversationKey,
  parseProviderFeatures,
  providerFeatureView,
  type WireAgentTarget,
} from "../shared/protocol";

describe("web protocol", () => {
  test("keeps the terminal subprotocol in one place", () => {
    expect(WEB_PROTOCOL).toBe("yolo.web.terminal.v1");
  });

  test("parses complete provider facts and rejects partial rows", () => {
    expect(
      parseProviderFeatures({
        conversation: true,
        submit_prompt: true,
        interrupt: true,
        interaction: "typed",
      }),
    ).toEqual({
      conversation: true,
      submit_prompt: true,
      interrupt: true,
      interaction: "typed",
    });
    expect(parseProviderFeatures({ interaction: "typed" })).toBeUndefined();
    expect(
      parseProviderFeatures({
        conversation: true,
        submit_prompt: true,
        interrupt: true,
        interaction: "future",
      }),
    ).toBeUndefined();
  });

  test("derives one conversation key from the browser target", () => {
    const target: WireAgentTarget = {
      machine: "machine-a",
      agent_id: "agent-a",
      execution_id: "execution-a",
      terminal_id: "terminal-a",
      runtime_epoch: "epoch-a",
    };
    expect(conversationKey(target)).toBe(
      "agent-a:execution-a:terminal-a:epoch-a",
    );
  });

  test("normalizes provider facts for browser presentation", () => {
    expect(
      providerFeatureView({
        conversation: true,
        submit_prompt: false,
        interrupt: true,
        interaction: "terminal",
      }),
    ).toEqual({
      conversation: true,
      submitPrompt: false,
      interrupt: true,
      interaction: "terminal",
    });
  });
});
