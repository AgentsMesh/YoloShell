import type { ArchiveOps } from "./archive";
import { getCatalogue } from "./catalogueService";
import { archive, read } from "./hub";

type Machine = { hub: string; machine: string; label: string; reachable: boolean };
type Read = typeof read;
type Archive = typeof archive;

/**
 * Hub-backed archive data access. `read`/`archive` only choose a pool lane, so
 * every call must carry the destination Hub explicitly; otherwise a peer's
 * archive read would be answered by the entry Hub.
 */
export function archiveOps(
  readAt: Read = read,
  archiveAt: Archive = archive,
  machines: () => Machine[] = () => getCatalogue().machines(),
  latest: () => unknown = () => getCatalogue().latest(),
): ArchiveOps {
  return {
    machines,
    latest,
    list: (hub) =>
      readAt(hub, async (client) => {
        const hello = (await client.request("hello", {}, hub)) as {
          hello?: { methods?: unknown };
        };
        const methods = Array.isArray(hello.hello?.methods) ? hello.hello.methods : [];
        if (!methods.includes("read_terminal_history")) {
          return { terminals: [], supports: false };
        }
        const reply = (await client.listArchived(hub)) as { terminals?: unknown[] };
        return { terminals: reply.terminals, supports: true };
      }),
    history: (hub, request) =>
      archiveAt(hub, (client) => client.readTerminalHistory(request, hub)),
  };
}

export const live = archiveOps();
