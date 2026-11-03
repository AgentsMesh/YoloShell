import { HubError } from "../../sdk/src/hub.ts";
import { HubTimeoutError } from "../../sdk/src/hubWire";
import { ApiError } from "./errors";
import { create } from "./hub";
import { machineFor } from "./machineHandles";
import { prepare, spec, type Kind, type Prepared } from "./creationRequest";

type Address = { hub_id: string; terminal_id: string };
type Created = { address: Address; creation_capability?: unknown };
type Started = { address: Address; provider_id?: string; binding?: string };

export type CreationOps = {
  request(
    method: string,
    params: Record<string, unknown>,
    destination: string,
    timeoutMs?: number,
  ): Promise<unknown>;
};

export const live: CreationOps = {
  request: (method, params, destination, timeoutMs) =>
    create(destination, (hub) => hub.request(method, params, destination, timeoutMs)),
};

const CERTAIN = new Set([
  "unsupported",
  "invalid_directory",
  "invalid_provider",
  "unauthorized",
  "conflict",
]);

/** API failures carry whether the Hub could have performed any side effect. */
export class CreationError extends ApiError {
  constructor(
    status: number,
    message: string,
    readonly uncertain: boolean,
  ) {
    super(status, message);
  }
}

function created(reply: unknown): Created {
  const value = (reply as { creation?: Created } | null)?.creation;
  if (!value?.address?.hub_id || !value.address.terminal_id) {
    throw new CreationError(502, "The Hub returned no terminal address", true);
  }
  return value;
}

function started(reply: unknown): Started {
  const value = (reply as { start?: Started } | null)?.start;
  if (!value?.address?.hub_id || !value.address.terminal_id) {
    throw new CreationError(502, "The Hub returned no terminal address", true);
  }
  return value;
}

export function terminalResult(address: Address): unknown {
  return {
    terminal: { machine: machineFor(address.hub_id), terminal_id: address.terminal_id },
  };
}

function detail(code: unknown, kind: Kind): { status: number; message: string } {
  if (code === "unsupported") {
    const message = kind === "agent" ? "That agent provider is not available on the selected machine" : "This Hub does not support creating that session";
    return { status: 409, message };
  }
  if (code === "invalid_directory") {
    return { status: 400, message: "That directory is unavailable on the selected machine" };
  }
  if (code === "unauthorized") {
    return { status: 400, message: "This plugin is not allowed to create sessions" };
  }
  if (code === "invalid_provider") {
    return { status: 400, message: "That agent provider is not available" };
  }
  if (code === "conflict") {
    return { status: 409, message: "A session with that name already exists on this machine" };
  }
  return { status: 502, message: "The Hub rejected the creation request" };
}

function unsupported(kind: Kind): CreationError {
  return new CreationError(
    409,
    kind === "agent"
      ? "This machine does not support starting agents yet. Pick another machine or create a shell."
      : "This machine does not support creating shells yet.",
    false,
  );
}

/** Only a definite protocol rejection may be treated as "no effect". */
function failure(error: unknown, kind: Kind, uncertain = false): never {
  if (error instanceof CreationError) throw error;
  if (error instanceof HubTimeoutError) {
    throw new CreationError(
      504,
      "The machine did not answer in time; the session may still be starting. Check Agents and Shells before retrying.",
      true,
    );
  }
  if (error instanceof HubError) {
    if (error.code === -32601) throw unsupported(kind);
    const code = (error.data as { code?: unknown } | null)?.code;
    const known = typeof code === "string" && CERTAIN.has(code);
    const definite = !uncertain && (error.code === -32602 || known);
    const value = detail(code, kind);
    throw new CreationError(value.status, value.message, !definite);
  }
  throw new CreationError(
    502,
    "The machine did not confirm the creation. Check Agents and Shells before retrying.",
    true,
  );
}

function finalizeFailure(error: unknown): never {
  if (error instanceof CreationError) throw error;
  throw new CreationError(
    502,
    "The session was created but its final state is not confirmed. Check Shells before retrying.",
    true,
  );
}

export async function finalizeShell(
  prepared: Prepared,
  reply: unknown,
  ops: CreationOps,
): Promise<unknown> {
  const creation = created(reply);
  try {
    await ops.request(
      "finalize",
      { creation_token: prepared.request.creation_token, creation },
      prepared.origin,
      20_000,
    );
  } catch (error) {
    return finalizeFailure(error);
  }
  return terminalResult(creation.address);
}

export async function dispatchShell(
  prepared: Prepared,
  ops: CreationOps,
): Promise<unknown> {
  let creation: Created;
  try {
    creation = created(
      await ops.request("create", { request: prepared.request }, prepared.origin, 45_000),
    );
  } catch (error) {
    return failure(error, "shell");
  }
  return finalizeShell(prepared, { creation }, ops);
}

/** AgentStart is idempotent by start_token and creates its own terminal. */
export async function dispatchAgent(
  prepared: Prepared,
  ops: CreationOps,
): Promise<unknown> {
  try {
    const start = started(
      await ops.request("agent_start", { request: prepared.request }, prepared.origin, 45_000),
    );
    return {
      terminal: { machine: machineFor(start.address.hub_id), terminal_id: start.address.terminal_id },
      provider: start.provider_id ?? prepared.provider ?? "unknown",
      binding: start.binding ?? "awaiting",
    };
  } catch (error) {
    return failure(error, "agent");
  }
}

/** Direct callers get a fresh intent; the server job path supplies its own. */
export async function createShell(
  body: Record<string, unknown>,
  ops: CreationOps = live,
): Promise<unknown> {
  return dispatchShell(prepare(spec("shell", body)), ops);
}

export async function startAgent(
  body: Record<string, unknown>,
  ops: CreationOps = live,
): Promise<unknown> {
  return dispatchAgent(prepare(spec("agent", body)), ops);
}
