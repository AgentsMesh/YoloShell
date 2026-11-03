import { HubError } from "../../sdk/src/hub.ts";
import { HubTimeoutError } from "../../sdk/src/hubWire";
import { live } from "./archiveSource";
import { ApiError } from "./errors";
import { terminalView } from "./machineView";
import { record, uuid } from "./validate";

const MAX_HISTORY = 4 * 1024 * 1024;
const MAX_ENCODED = Math.ceil((MAX_HISTORY * 4) / 3) + 8;
const MAX_ARCHIVED = 500;

type Machine = { hub: string; machine: string; label: string; reachable: boolean };

export type ArchiveOps = {
  machines(): Machine[];
  list(hub: string): Promise<{ terminals?: unknown[]; supports?: boolean }>;
  history(hub: string, request: Record<string, unknown>): Promise<unknown>;
  latest?(): unknown;
};

function lookup(machine: unknown, ops: ArchiveOps): Machine {
  if (typeof machine !== "string") throw new ApiError(400, "Invalid machine");
  const found = ops.machines().find((item) => item.machine === machine);
  if (!found) throw new ApiError(400, "Unknown machine");
  return found;
}

/** Archived rows are fetched on demand; the live catalogue never carries them. */
export async function listArchive(
  machine?: string,
  ops: ArchiveOps = live,
): Promise<unknown> {
  const known = ops.machines();
  const selected = machine === undefined ? known : known.filter((item) => item.machine === machine);
  if (machine !== undefined && selected.length === 0) {
    throw new ApiError(400, "Unknown machine");
  }
  const terminals: unknown[] = [];
  const unsupported: { machine: string; label: string }[] = [];
  let reachable = 0;
  let truncated = false;
  let finished = 0;
  let archived = 0;
  for (const item of selected) {
    if (!item.reachable) continue;
    reachable += 1;
    try {
      const reply = await ops.list(item.hub);
      if (reply.supports === false) {
        unsupported.push({ machine: item.machine, label: item.label });
        continue;
      }
      for (const value of reply.terminals ?? []) {
        if (terminals.length >= MAX_ARCHIVED) {
          truncated = true;
          break;
        }
        terminals.push(terminalView(value, item.hub));
        archived += 1;
      }
    } catch (error) {
      // One unreachable machine must not hide another machine's archive.
      if (error instanceof HubError && (error.code === -32601 || error.code === -32602)) {
        unsupported.push({ machine: item.machine, label: item.label });
      }
    }
    if (truncated) break;
  }
  const scope = new Set(selected.filter((item) => item.reachable).map((item) => item.machine));
  const seen = new Set(
    (terminals as { address?: { machine?: string; terminal_id?: string } }[]).map(
      (row) => `${row.address?.machine}:${row.address?.terminal_id}`,
    ),
  );
  const live = (ops.latest?.() ?? null) as {
    terminals?: { terminals?: Record<string, any>[] };
  } | null;
  for (const value of live?.terminals?.terminals ?? []) {
    if (terminals.length >= MAX_ARCHIVED) {
      truncated = true;
      break;
    }
    if (!scope.has(value.address?.machine)) continue;
    const key = `${value.address?.machine}:${value.address?.terminal_id}`;
    if (seen.has(key)) continue;
    const status = value.runtime?.status;
    if (value.archived === true || (status !== "exited" && status !== "lost")) continue;
    terminals.push({ ...value, finished: true });
    seen.add(key);
    finished += 1;
  }
  return {
    machines: selected.map(({ machine: handle, label, reachable: live }) => ({
      machine: handle,
      label,
      reachable: live,
    })),
    terminals,
    reachable,
    unsupported,
    truncated,
    archived,
    finished,
  };
}

export type ArchivedHistory = {
  data: Uint8Array;
  fromSequence: number;
  nextSequence: number;
  truncated: boolean;
};

function number(value: unknown, field: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    throw new ApiError(502, `The machine returned an invalid ${field}`);
  }
  return value;
}

function failure(error: unknown): never {
  if (error instanceof HubTimeoutError) {
    throw new ApiError(504, "The machine did not return archived history in time");
  }
  if (error instanceof HubError) {
    const code = (error.data as { code?: unknown } | null)?.code;
    if (error.code === -32601 || code === "unsupported") {
      throw new ApiError(409, "This machine does not keep archived history");
    }
    if (code === "not_found") throw new ApiError(404, "That archived session is unavailable");
    if (code === "conflict") throw new ApiError(409, "That archived session changed");
    if (error.code === -32602) throw new ApiError(400, "Invalid archive history request");
    throw new ApiError(502, "The machine could not read the archive");
  }
  if (error instanceof ApiError) throw error;
  throw new ApiError(502, "The machine did not return archive history");
}

/** Read one archived runtime's retained raw output stream. */
export async function readArchivedHistory(
  body: Record<string, unknown>,
  ops: ArchiveOps = live,
): Promise<ArchivedHistory> {
  const item = record(body);
  const machine = lookup(item.machine, ops);
  const terminalId = uuid(item.terminal_id);
  const runtimeEpoch = uuid(item.runtime_epoch);
  const requested = item.max_bytes;
  if (
    requested !== undefined &&
    (typeof requested !== "number" || !Number.isSafeInteger(requested) || requested < 1)
  ) {
    throw new ApiError(400, "Invalid history size");
  }
  const maxBytes =
    typeof requested === "number" ? Math.min(requested, MAX_HISTORY) : undefined;
  let reply: unknown;
  try {
    reply = await ops.history(machine.hub, {
      schema_version: 1,
      target: {
        address: { hub_id: machine.hub, terminal_id: terminalId },
        runtime_epoch: runtimeEpoch,
      },
      ...(maxBytes === undefined ? {} : { max_bytes: maxBytes }),
    });
  } catch (error) {
    failure(error);
  }
  const history = (reply as { history?: Record<string, unknown> } | null)?.history;
  if (!history || typeof history.data !== "string" || history.data.length > MAX_ENCODED) {
    throw new ApiError(502, "The machine returned no archive history");
  }
  const data = Buffer.from(history.data, "base64");
  if (data.byteLength > MAX_HISTORY) {
    throw new ApiError(502, "The machine returned too much archive history");
  }
  return {
    data,
    fromSequence: number(history.from_sequence, "history start"),
    nextSequence: number(history.next_sequence, "history end"),
    truncated: history.truncated === true,
  };
}
