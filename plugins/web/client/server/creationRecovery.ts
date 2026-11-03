import { HubError } from "../../sdk/src/hub.ts";
import { HubTimeoutError } from "../../sdk/src/hubWire";
import {
  CreationError,
  finalizeShell,
  terminalResult,
  type CreationOps,
} from "./creation";
import type { Prepared } from "./creationRequest";

type Address = { hub_id: string; terminal_id: string };
type Creation = { address?: Address; creation_capability?: unknown };
type Outcome = {
  status?: string;
  creation?: Creation;
  address?: Address;
};
type Query = { creation_token?: string; outcome?: Outcome };
export type Recovery =
  | { state: "settled"; result: unknown }
  | { state: "unconfirmed"; error: string };

function address(value: Address | undefined): Address | null {
  if (!value?.hub_id || !value.terminal_id) return null;
  return value;
}

function settled(value: Address, archived = false): Recovery {
  return {
    state: "settled",
    result: { ...(terminalResult(value) as object), ...(archived ? { archived: true } : {}) },
  };
}

function message(error: unknown): string {
  if (error instanceof HubTimeoutError) {
    return "The machine did not answer the confirmation check; the session may still be starting.";
  }
  if (error instanceof HubError && error.code === -32601) {
    return "This machine cannot confirm the creation. Check Agents and Shells before retrying.";
  }
  if (error instanceof CreationError) return error.message;
  return "The machine could not confirm the creation. Check Agents and Shells before retrying.";
}

/**
 * QueryCreation inspects durable evidence. It never replays Create, so an
 * uncertain or missing result must stay unresolved instead of starting again.
 */
export async function recoverShell(
  prepared: Prepared,
  ops: CreationOps,
): Promise<Recovery> {
  let reply: unknown;
  try {
    reply = await ops.request(
      "query_creation",
      { request: prepared.request },
      prepared.origin,
      10_000,
    );
  } catch (error) {
    return { state: "unconfirmed", error: message(error) };
  }
  const query = (reply as { query?: Query } | null)?.query;
  if (
    !query ||
    query.creation_token !== prepared.request.creation_token ||
    !query.outcome
  ) {
    return {
      state: "unconfirmed",
      error: "The machine returned an unrecognized creation result. Check Agents and Shells before retrying.",
    };
  }
  const outcome = query.outcome;
  if (outcome.status === "ready") {
    try {
      return {
        state: "settled",
        result: await finalizeShell(prepared, { creation: outcome.creation }, ops),
      };
    } catch (error) {
      return {
        state: "unconfirmed",
        error:
          error instanceof Error
            ? error.message
            : "The session was created but is not finalized yet. Check Shells before retrying.",
      };
    }
  }
  if (outcome.status === "finalized" || outcome.status === "survivor") {
    const value = address(outcome.creation?.address);
    if (value) return settled(value);
  }
  if (outcome.status === "archived") {
    const value = address(outcome.address);
    if (value) return settled(value, true);
  }
  if (outcome.status === "missing" || outcome.status === "pending") {
    return {
      state: "unconfirmed",
      error:
        "The machine has not confirmed this session yet; it may still be starting. Do not resubmit a new request.",
    };
  }
  if (outcome.status === "unknown") {
    return {
      state: "unconfirmed",
      error:
        "The machine cannot confirm whether this session was created. Check Agents and Shells before retrying.",
    };
  }
  return {
    state: "unconfirmed",
    error: "The machine returned an unrecognized creation result. Check Agents and Shells before retrying.",
  };
}
