import { listArchive, readArchivedHistory } from "./archive";
import { getJob } from "./creationJobs";
import { inventory } from "./inventory";
import { layout } from "./layoutView";
import { hubFor } from "./machineHandles";
import { sanitize } from "./sanitize";
import { terminalLog } from "./terminalRegistry";
import { uuid } from "./validate";

const MAX_LOG = 8 * 1024 * 1024;

function query(value: string | null): string | undefined {
  return value === null ? undefined : value;
}

/** Authenticated GET API. Returns null for a non-API or unknown path. */
export async function apiGet(url: URL, authorized: boolean): Promise<Response | null> {
  if (!url.pathname.startsWith("/api/")) return null;
  if (!authorized) return new Response("unauthorized", { status: 401 });
  if (url.pathname === "/api/inventory") {
    return Response.json(sanitize(await inventory()));
  }
  const job = url.pathname.match(/^\/api\/jobs\/([0-9a-f-]{36})$/);
  if (job) return Response.json(sanitize(getJob(job[1])));
  if (url.pathname === "/api/layout") {
    return Response.json(sanitize(await layout()));
  }
  if (url.pathname === "/api/archive") {
    return Response.json(sanitize(await listArchive(query(url.searchParams.get("machine")))));
  }
  if (url.pathname === "/api/archive/history") {
    const max = url.searchParams.get("max_bytes");
    const history = await readArchivedHistory({
      machine: query(url.searchParams.get("machine")),
      terminal_id: query(url.searchParams.get("terminal_id")),
      runtime_epoch: query(url.searchParams.get("runtime_epoch")),
      max_bytes: max === null ? undefined : Number(max),
    });
    return new Response(history.data, {
      headers: {
        "content-type": "application/octet-stream",
        "x-archive-truncated": String(history.truncated),
        "x-archive-from-sequence": String(history.fromSequence),
        "x-archive-next-sequence": String(history.nextSequence),
      },
    });
  }
  if (url.pathname === "/api/terminal/log") {
    const hub = hubFor(query(url.searchParams.get("machine")) ?? "");
    const target = {
      address: {
        hub_id: hub,
        terminal_id: uuid(query(url.searchParams.get("terminal_id")) ?? ""),
      },
      runtime_epoch: uuid(query(url.searchParams.get("runtime_epoch")) ?? ""),
    };
    const log = terminalLog(target, MAX_LOG);
    if (!log) return new Response("no terminal log", { status: 404 });
    return new Response(log.data, {
      headers: {
        "content-type": "application/octet-stream",
        "x-log-truncated": String(log.truncated),
      },
    });
  }
  return null;
}
