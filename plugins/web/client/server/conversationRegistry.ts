import { ConversationActor } from "./conversationActor";
import type { TerminalTarget } from "./target";
import type { SessionConversationFrame } from "../shared/protocol";
import { conversationKey as wireConversationKey } from "../shared/protocol";

export type Binding = {
  agent: { address: { hub_id: string; agent_id: string }; execution_id: string };
  runtime: TerminalTarget;
};
export type Subscriber = { send(frame: SessionConversationFrame): void };

export function conversationKey(binding: Binding): string {
  return wireConversationKey({
    agent_id: binding.agent.address.agent_id,
    execution_id: binding.agent.execution_id,
    terminal_id: binding.runtime.address.terminal_id,
    runtime_epoch: binding.runtime.runtime_epoch,
  });
}

const actors = new Map<string, ConversationActor>();

export function watchConversation(
  binding: Binding,
  subscriber: Subscriber,
): { key: string; stop(): void } {
  const key = conversationKey(binding);
  const raw = `${binding.agent.address.hub_id}:${key}`;
  let actor = actors.get(raw);
  if (!actor) {
    actor = new ConversationActor(key, binding, () => {
      if (actors.get(raw) === actor) actors.delete(raw);
    });
    actors.set(raw, actor);
  }
  actor.add(subscriber);
  return {
    key,
    stop: () => actor.remove(subscriber),
  };
}
