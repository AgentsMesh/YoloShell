/** Web plugin protocol shared by the Bun server and the browser modules. */
export const WEB_PROTOCOL = "yolo.web.terminal.v1";

export type ProviderInteraction = "none" | "terminal" | "typed";

/** Provider facts as carried on the inventory wire (snake_case). */
export type ProviderFeatures = {
  conversation: boolean;
  submit_prompt: boolean;
  interrupt: boolean;
  interaction: ProviderInteraction;
};

/** Provider facts normalized for browser presentation. */
export type ProviderFeaturesView = {
  conversation: boolean;
  submitPrompt: boolean;
  interrupt: boolean;
  interaction: ProviderInteraction;
};

export function parseProviderFeatures(value: unknown): ProviderFeatures | undefined {
  if (typeof value !== "object" || value === null) return undefined;
  const item = value as Record<string, unknown>;
  if (
    typeof item.conversation !== "boolean" ||
    typeof item.submit_prompt !== "boolean" ||
    typeof item.interrupt !== "boolean" ||
    (item.interaction !== "none" &&
      item.interaction !== "terminal" &&
      item.interaction !== "typed")
  ) {
    return undefined;
  }
  return {
    conversation: item.conversation,
    submit_prompt: item.submit_prompt,
    interrupt: item.interrupt,
    interaction: item.interaction,
  };
}

export function providerFeatureView(features: ProviderFeatures): ProviderFeaturesView {
  return {
    conversation: features.conversation,
    submitPrompt: features.submit_prompt,
    interrupt: features.interrupt,
    interaction: features.interaction,
  };
}

export type WireTerminalTarget = {
  machine: string;
  terminal_id: string;
  runtime_epoch: string;
};

/** Browser request shape; the server resolves `machine` to its Hub binding. */
export type WireAgentTarget = {
  machine?: string;
  agent_id: string;
  execution_id: string;
  terminal_id: string;
  runtime_epoch: string;
};

export type WireLeaseState =
  | "observing"
  | "acquiring"
  | "controlled"
  | "lost"
  | "released";

export function conversationKey(
  target: Pick<WireAgentTarget, "agent_id" | "execution_id" | "terminal_id" | "runtime_epoch">,
): string {
  return [
    target.agent_id,
    target.execution_id,
    target.terminal_id,
    target.runtime_epoch,
  ].join(":");
}

export type SessionClientFrame =
  | { type: "hello"; token: string; client_id: string }
  | { type: "catalogue.refresh" }
  | { type: "catalogue.snapshot" }
  | { type: "conversation.watch"; binding: WireAgentTarget }
  | { type: "conversation.unwatch" };

export type SessionConversationFrame =
  | { type: "conversation"; key: string; reading: unknown }
  | { type: "conversation.error"; key?: string; message: string }
  | { type: "conversation.unavailable"; key: string; reason: string };

export type SessionServerFrame =
  | { type: "ready"; client_id: string }
  | { type: "catalogue"; inventory: unknown }
  | { type: "catalogue.patch"; patch: unknown }
  | SessionConversationFrame;

export type TerminalClientFrame =
  | {
      type: "hello";
      token: string;
      client_id: string;
      target: WireTerminalTarget;
    }
  | { type: "take" }
  | { type: "release" }
  | { type: "resize"; cols: number; rows: number }
  | { type: "ping" };

export type TerminalOutputFrame =
  | { type: "ready"; cols?: number; rows?: number; seed?: string }
  | { type: "live" }
  | { type: "reset" }
  | { type: "exit"; code?: number | null; signal?: number | null }
  | { type: "unreachable" }
  | { type: "error"; message: string };

export type TerminalControlFrame =
  | { type: "ready" }
  | { type: "error"; message: string }
  | {
      type: "lease";
      state: WireLeaseState;
      controller: boolean;
      controlled: boolean;
      viewers: number;
    }
  | { type: "presence"; viewers: number; controlled: boolean; mine: boolean };

export type TerminalServerFrame =
  | TerminalOutputFrame
  | TerminalControlFrame
  | { type: "pong" };
