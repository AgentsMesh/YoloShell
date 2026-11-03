import { interact } from "./hub";
import { AGENT_SCHEMA } from "./schema";
import { terminalTarget } from "./target";
import { releaseControlForPrompt } from "./terminalRegistry";
import { agentTarget, clientIdentity, message, record } from "./validate";

export type PromptOps = {
  send(request: unknown, destination: string): Promise<unknown>;
  release?(body: Record<string, unknown>, clientId: string): Promise<void>;
};

const live: PromptOps = {
  send: (request, destination) =>
    interact(destination, (hub) => hub.agentPrompt(request, destination)),
  release: (body, client) =>
    releaseControlForPrompt(terminalTarget(body), client),
};

export async function prompt(
  body: Record<string, unknown>,
  ops: PromptOps = live,
): Promise<unknown> {
  const { agent } = agentTarget(body);
  const client = clientIdentity(record(body).client_id);
  if (
    client !== null &&
    body.terminal_id !== undefined &&
    body.runtime_epoch !== undefined
  ) {
    await ops.release?.(body, client);
  }
  const request = {
    schema_version: AGENT_SCHEMA,
    request_id: crypto.randomUUID(),
    reference: { reference: "execution", target: agent },
    prompt: message(body.text),
    wait: { wait: "none" },
    take_control: true,
  };
  for (let attempt = 0; ; attempt += 1) {
    try {
      return await ops.send(request, agent.address.hub_id);
    } catch (error) {
      // A detach and its Hub lease release can cross this request. Retry only
      // the explicit same-tab handoff; another controller remains a conflict.
      if (client === null || !conflict(error) || attempt >= 7) throw error;
      await ops.release?.(body, client);
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
}

function conflict(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    (error as { data?: { code?: unknown } }).data?.code === "conflict"
  );
}
