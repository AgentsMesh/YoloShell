import { ApiError } from "./errors";
import { hubId, interact, read } from "./hub";
import { AGENT_SCHEMA } from "./schema";
import { agentTarget, clientIdentity, record, uuid } from "./validate";
import { releaseControlForPrompt } from "./terminalRegistry";
import type { TerminalTarget } from "./target";

const CHOICE = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/;

type Snapshot = {
  schema_version?: number;
  cursor?: unknown;
  terminal?: { address?: { terminal_id?: string }; runtime_epoch?: string };
  lifecycle?: { state?: string };
  reachability?: string;
  capabilities?: { respond_interaction?: { support?: string } };
  interaction?: {
    reference?: { target?: unknown; id?: string };
    choices?: { id?: string }[];
  } | null;
};

export type RespondOps = {
  agents(): Promise<unknown>;
  observeLease(target: unknown, destination: string): Promise<unknown>;
  lease(request: unknown, destination: string): Promise<unknown>;
  control(request: unknown, grant: unknown, destination: string): Promise<unknown>;
  release(grant: unknown, destination: string): Promise<unknown>;
  releaseControl?(runtime: TerminalTarget, clientId: string): Promise<void>;
};

const live: RespondOps = {
  agents: () => read(hubId(), (hub) => hub.agents()),
  observeLease: (target, destination) =>
    interact(destination, (hub) => hub.observeLease(target, destination)),
  lease: (request, destination) => interact(destination, (hub) => hub.lease(request, destination)),
  control: (request, grant, destination) =>
    interact(destination, (hub) => hub.agentControl(request, grant, destination)),
  release: (grant, destination) => interact(destination, (hub) => hub.release(grant, destination)),
  releaseControl: (runtime, client) => releaseControlForPrompt(runtime, client),
};

/** Answer one current interaction after re-reading the live snapshot. */
export async function respond(
  body: Record<string, unknown>,
  ops: RespondOps = live,
): Promise<unknown> {
  const { agent, runtime } = agentTarget(body);
  const destination = agent.address.hub_id;
  const interactionId = uuid(record(body).interaction_id);
  const choice = record(body).choice;
  if (typeof choice !== "string" || !CHOICE.test(choice)) {
    throw new ApiError(400, "Invalid choice");
  }
  const reply = (await ops.agents()) as { agents?: Snapshot[] };
  const snapshot = (reply.agents ?? []).find(
    (candidate) =>
      (candidate.cursor as { target?: { address?: { agent_id?: string }; execution_id?: string } })
        ?.target?.address?.agent_id === agent.address.agent_id &&
      (candidate.cursor as { target?: { execution_id?: string } })?.target?.execution_id ===
        agent.execution_id,
  );
  if (!snapshot?.cursor || snapshot.lifecycle?.state !== "running") {
    throw new ApiError(409, "The agent is no longer current");
  }
  if (snapshot.capabilities?.respond_interaction?.support !== "available") {
    throw new ApiError(409, "This agent cannot answer the interaction");
  }
  const interaction = snapshot.interaction;
  if (!interaction || interaction.reference?.id !== interactionId) {
    throw new ApiError(409, "The interaction is no longer current");
  }
  if (!interaction.choices?.some((candidate) => candidate.id === choice)) {
    throw new ApiError(400, "Choice is not available");
  }
  if (
    snapshot.terminal?.address?.terminal_id !== runtime.address.terminal_id ||
    snapshot.terminal?.runtime_epoch !== runtime.runtime_epoch
  ) {
    throw new ApiError(409, "The interaction terminal changed");
  }
  const client = clientIdentity(record(body).client_id);
  if (client !== null) await ops.releaseControl?.(runtime, client);
  const grant = await acquire(runtime, destination, ops);
  try {
    const request = {
      schema_version: snapshot.schema_version ?? AGENT_SCHEMA,
      request_id: crypto.randomUUID(),
      target: agent,
      expected_cursor: snapshot.cursor,
      action: {
        operation: "respond",
        interaction: { target: agent, id: interactionId },
        response: { kind: "choice", choice },
      },
    };
    return await ops.control(request, grant, destination);
  } finally {
    await ops.release(grant, destination).catch(() => undefined);
  }
}

async function acquire(
  runtime: unknown,
  destination: string,
  ops: RespondOps,
): Promise<unknown> {
  const principal = {
    client_id: crypto.randomUUID(),
    controller_instance_id: crypto.randomUUID(),
  };
  const observed = (await ops.observeLease(runtime, destination)) as {
    lease?: { generation?: number } | null;
  };
  const generation = observed.lease?.generation;
  const mode =
    typeof generation === "number"
      ? { mode: "take_control", observed_generation: generation }
      : { mode: "acquire" };
  const reply = (await ops.lease({
    request_id: crypto.randomUUID(),
    target: runtime,
    principal,
    mode,
  }, destination)) as { grant?: unknown };
  if (!reply.grant) throw new ApiError(409, "Writer lease is unavailable");
  return reply.grant;
}
