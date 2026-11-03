import {
  CreationError,
  dispatchAgent,
  dispatchShell,
  type CreationOps,
} from "./creation";
import { recoverShell } from "./creationRecovery";
import type { CreationIntent } from "./creationIntent";
import type { Prepared } from "./creationRequest";

function prepared(intent: CreationIntent): Prepared {
  return {
    kind: intent.kind,
    origin: intent.origin,
    request: intent.request,
    ...(intent.provider ? { provider: intent.provider } : {}),
  };
}

function settled(intent: CreationIntent, result: unknown): CreationIntent {
  return {
    ...intent,
    state: "settled",
    result,
    error: undefined,
    updatedAt: Date.now(),
  };
}

function unconfirmed(intent: CreationIntent, error: string): CreationIntent {
  return { ...intent, state: "unconfirmed", error, updatedAt: Date.now() };
}

function failed(intent: CreationIntent, error: string): CreationIntent {
  return { ...intent, state: "failed", error, result: undefined, updatedAt: Date.now() };
}

function reason(error: unknown): string {
  if (error instanceof CreationError) return error.message;
  return "The machine did not confirm the creation. Check Agents and Shells before retrying.";
}

/**
 * One dispatch or recovery attempt. A fresh intent checkpoints `dispatched`
 * through the caller before any Hub mutation, so a crash can never replay Create.
 */
export async function runIntent(
  intent: CreationIntent,
  ops: CreationOps,
  checkpoint: (value: CreationIntent) => void = () => undefined,
): Promise<CreationIntent> {
  const fresh = intent.state === "prepared";
  const next = {
    ...intent,
    attempts: intent.attempts + 1,
    updatedAt: Date.now(),
  };
  if (fresh) {
    next.state = "dispatched";
    checkpoint(next);
  }
  const request = prepared(next);
  try {
    if (next.kind === "agent") {
      return settled(next, await dispatchAgent(request, ops));
    }
    if (fresh) {
      return settled(next, await dispatchShell(request, ops));
    }
    const recovery = await recoverShell(request, ops);
    return recovery.state === "settled"
      ? settled(next, recovery.result)
      : unconfirmed(next, recovery.error);
  } catch (error) {
    const uncertain = !(error instanceof CreationError) || error.uncertain;
    return uncertain ? unconfirmed(next, reason(error)) : failed(next, reason(error));
  }
}
