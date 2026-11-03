import { HubError } from "../../sdk/src/hub.ts";
import { ApiError } from "./errors";
import { read, watch } from "./hub";
import { agentTarget, cursor, digest } from "./validate";

type Reading = { reading?: unknown };

function unavailable(error: unknown): { available: false; reason: string } | null {
  if (!(error instanceof HubError) || error.code !== -32001) return null;
  const data = error.data as { error?: unknown } | null;
  return {
    available: false,
    reason: typeof data?.error === "string" ? data.error : "conversation_unavailable",
  };
}

export async function conversation(body: Record<string, unknown>): Promise<unknown> {
  const binding = agentTarget(body);
  const destination = binding.agent.address.hub_id;
  let address: unknown;
  try {
    const reply = (await read(destination, (hub) =>
      hub.resolveConversation(binding, destination),
    )) as {
      conversation?: { address?: unknown };
    };
    address = reply.conversation?.address;
    if (!address) throw new ApiError(502, "Conversation descriptor is missing");
  } catch (error) {
    const missing = unavailable(error);
    if (missing) return missing;
    throw error;
  }
  const request = {
    address,
    binding,
    cursor: cursor(body.cursor, destination),
    limits: { entries: 64, bytes: 262_144 },
    since_revision: digest(body.since_revision),
    since_generation: digest(body.since_generation),
  };
  try {
    const reply = (await watch(destination, (hub) =>
      hub.watchConversation(request, 1000, destination),
    )) as Reading;
    if (!reply.reading) throw new ApiError(502, "Conversation read is missing");
    return { available: true, reading: reply.reading };
  } catch (error) {
    const missing = unavailable(error);
    if (missing) return missing;
    throw error;
  }
}
