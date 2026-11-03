import type { ServerWebSocket } from "bun";
import type { TerminalTarget } from "./target";
import { TerminalActor } from "./terminalActor";

export type OutputHandle = { close(): void };
export type ControlHandle = {
  message(message: string | Uint8Array): void;
  close(): void;
};

const actors = new Map<string, TerminalActor>();

function key(target: TerminalTarget): string {
  return `${target.address.hub_id}:${target.address.terminal_id}:${target.runtime_epoch}`;
}

function get(target: TerminalTarget): TerminalActor {
  const id = key(target);
  let actor = actors.get(id);
  if (!actor) {
    actor = new TerminalActor(target, () => {
      if (actors.get(id) === actor) actors.delete(id);
    });
    actors.set(id, actor);
  }
  return actor;
}

export function attachOutput(
  ws: ServerWebSocket<unknown>,
  target: TerminalTarget,
): OutputHandle {
  const actor = get(target);
  actor.attachOutput(ws);
  return { close: () => actor.detachOutput(ws) };
}

export function attachControl(
  ws: ServerWebSocket<unknown>,
  target: TerminalTarget,
  clientId: string,
): ControlHandle {
  const actor = get(target);
  actor.attachControl(ws, clientId);
  return {
    message: (message) => actor.message(ws, clientId, message),
    close: () => actor.detachControl(ws, clientId),
  };
}

export function terminalActorCount(): number {
  return actors.size;
}

/** Release this tab's control before a prompt takes the writer lease. */
export async function releaseControlForPrompt(
  target: TerminalTarget,
  clientId: string,
): Promise<void> {
  await actors.get(key(target))?.releaseForPrompt(clientId);
}

/** The live actor's retained seed/ring; never creates an actor. */
export function terminalLog(
  target: TerminalTarget,
  limit: number,
): { data: Uint8Array; truncated: boolean } | null {
  return actors.get(key(target))?.history(limit) ?? null;
}

export function closeTerminals(): void {
  for (const actor of actors.values()) actor.close();
  actors.clear();
}
