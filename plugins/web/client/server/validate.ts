import { ApiError } from "./errors";
import { entryMachine, hubFor } from "./machineHandles";

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const MAX_PROMPT_BYTES = 65_536;

export function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new ApiError(400, "Request body must be a JSON object");
  }
  return value as Record<string, unknown>;
}

export function uuid(value: unknown): string {
  const text = typeof value === "string" ? value.toLowerCase() : "";
  if (!UUID.test(text)) throw new ApiError(400, "Invalid identifier");
  return text;
}

export function digest(value: unknown): number[] | null {
  if (value === null || value === undefined) return null;
  if (
    !Array.isArray(value) ||
    value.length !== 32 ||
    !value.every((item) => Number.isInteger(item) && item >= 0 && item < 256)
  ) {
    throw new ApiError(400, "Invalid digest");
  }
  return value as number[];
}

function requiredDigest(value: unknown): number[] {
  const parsed = digest(value);
  if (!parsed) throw new ApiError(400, "Digest is required");
  return parsed;
}

export function agentTarget(value: unknown) {
  const item = record(value);
  const machine = item.machine === undefined || item.machine === null
    ? entryMachine()
    : uuid(item.machine);
  const hub = hubFor(machine);
  return {
    agent: {
      address: { hub_id: hub, agent_id: uuid(item.agent_id) },
      execution_id: uuid(item.execution_id),
    },
    runtime: {
      address: { hub_id: hub, terminal_id: uuid(item.terminal_id) },
      runtime_epoch: uuid(item.runtime_epoch),
    },
  };
}
export function cursor(value: unknown, hub: string) {
  if (value === null || value === undefined) return null;
  const item = record(value);
  const address = record(item.address);
  if (!Number.isInteger(item.before_offset) || (item.before_offset as number) < 0) {
    throw new ApiError(400, "Invalid cursor");
  }
  return {
    address: {
      hub_id: hub,
      conversation_id: requiredDigest(address.conversation_id),
    },
    history_generation: requiredDigest(item.history_generation),
    before_offset: item.before_offset,
  };
}

export function message(value: unknown, maximum = MAX_PROMPT_BYTES): string {
  if (typeof value !== "string") throw new ApiError(400, "Message is required");
  if (value.trim().length === 0) throw new ApiError(400, "Message is empty");
  if (new TextEncoder().encode(value).length > maximum) {
    throw new ApiError(413, "Message is too long");
  }
  for (const character of value) {
    const code = character.codePointAt(0) ?? 0;
    if (code < 32 && character !== "\n" && character !== "\t") {
      throw new ApiError(400, "Message contains control characters");
    }
  }
  return value;
}

const CLIENT_ID = /^[A-Za-z0-9_-]{1,64}$/;

/** A browser tab identity is optional for older clients. */
export function clientIdentity(value: unknown): string | null {
  return typeof value === "string" && CLIENT_ID.test(value) ? value : null;
}
