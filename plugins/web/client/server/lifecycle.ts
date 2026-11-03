import { HubError } from "../../sdk/src/hub.ts";
import { HubTimeoutError } from "../../sdk/src/hubWire";
import { ApiError } from "./errors";
import { hubId, lifecycle } from "./hub";
import { hubFor, machineFor } from "./machineHandles";
import { invalidateInventory } from "./inventory";
import { record, uuid } from "./validate";

export type LifecycleOps = {
  request(
    method: string,
    params: Record<string, unknown>,
    destination: string,
    timeoutMs?: number,
  ): Promise<unknown>;
};

export type LifecycleAction = "terminate" | "archive";

const live: LifecycleOps = {
  request: (method, params, destination, timeoutMs) =>
    lifecycle(destination, (hub) => hub.request(method, params, destination, timeoutMs)),
};

type Summary = {
  address?: { hub_id?: string; terminal_id?: string };
  archived?: boolean;
  runtime?: { status?: string };
};

function destination(item: Record<string, unknown>): string {
  const machine = item.machine;
  if (machine === undefined || machine === null) return hubId();
  if (typeof machine !== "string") throw new ApiError(400, "Invalid machine");
  return hubFor(machine);
}

function expectedRuntime(value: unknown): string | null {
  if (value === undefined || value === null || value === "") return null;
  return uuid(value);
}

function action(item: Record<string, unknown>): { operation: LifecycleAction; force?: boolean } {
  if (item.action === "terminate") {
    return { operation: "terminate", force: item.force === true };
  }
  if (item.action === "archive") return { operation: "archive" };
  throw new ApiError(400, "Unknown lifecycle action");
}

function failure(error: unknown): never {
  if (error instanceof HubTimeoutError) {
    throw new ApiError(
      504,
      "The machine did not confirm the lifecycle change. Check the catalogue before retrying.",
    );
  }
  if (error instanceof HubError) {
    if (error.code === -32601) {
      throw new ApiError(409, "This machine does not support ending or archiving sessions");
    }
    const code = (error.data as { code?: unknown } | null)?.code;
    if (code === "target_mismatch") {
      throw new ApiError(409, "This session changed on the machine; refresh and try again");
    }
    if (code === "not_found") throw new ApiError(404, "That session no longer exists");
    if (code === "unauthorized") {
      throw new ApiError(403, "This plugin is not allowed to change sessions");
    }
    if (code === "conflict") {
      throw new ApiError(409, "The session is not in a state that allows that action");
    }
    if (code === "outcome_unknown") {
      throw new ApiError(504, "The machine could not confirm the lifecycle change");
    }
    if (code === "unsupported") {
      throw new ApiError(409, "This machine does not support that lifecycle action");
    }
    throw new ApiError(502, "The Hub rejected the lifecycle request");
  }
  throw new ApiError(502, "The machine did not confirm the lifecycle change");
}

/** Terminate or archive one exact terminal using the public Hub lifecycle command. */
export async function runLifecycle(
  body: Record<string, unknown>,
  ops: LifecycleOps = live,
  changed: () => void = invalidateInventory,
): Promise<unknown> {
  const item = record(body);
  const hub = destination(item);
  const terminalId = uuid(item.terminal_id);
  const requested = action(item);
  const command = {
    request_id: crypto.randomUUID(),
    selection: {
      address: { hub_id: hub, terminal_id: terminalId },
      expected_runtime: expectedRuntime(item.runtime_epoch),
    },
    action: requested,
  };
  let reply: unknown;
  try {
    reply = await ops.request("terminal", { command }, hub, 30_000);
  } catch (error) {
    return failure(error);
  }
  const summary = (reply as { terminal?: Summary } | null)?.terminal;
  if (!summary?.address?.terminal_id) {
    throw new ApiError(502, "The Hub returned no terminal state");
  }
  if (summary.address.hub_id && summary.address.hub_id !== hub) {
    throw new ApiError(502, "The Hub returned a terminal from another machine");
  }
  changed();
  return {
    terminal: {
      machine: machineFor(hub),
      terminal_id: summary.address.terminal_id,
      archived: summary.archived === true,
      status: summary.runtime?.status ?? null,
    },
  };
}
