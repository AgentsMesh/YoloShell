import { HubError, TerminalAttachment } from "../../sdk/src/index.ts";
import { hubEndpoint } from "./hub";
import type { TerminalTarget } from "./target";

export type Seed = {
  kind: "history" | "snapshot";
  cols: number;
  rows: number;
  data: Uint8Array;
};

/** History is an optimization; these failures fall back to the current screen. */
export function historyUnavailable(error: unknown): boolean {
  if (!(error instanceof HubError)) return false;
  if (error.code === -32602) return true;
  const code = (error.data as { code?: unknown } | null)?.code;
  return code === "outcome_unknown" || code === "unsupported" || code === "invalid_cursor";
}

function combine(initial: ReturnType<TerminalAttachment["takeInitial"]>): Seed | null {
  if (!initial) return null;
  if (initial.history) {
    return {
      kind: "history",
      cols: initial.history.cols,
      rows: initial.history.rows,
      data: initial.history.data,
    };
  }
  const chunks = [...(initial.snapshot ? [initial.snapshot.data] : []), ...initial.chunks];
  if (chunks.length === 0) return null;
  const total = chunks.reduce((sum, chunk) => sum + chunk.byteLength, 0);
  const data = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    data.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return {
    kind: "snapshot",
    cols: initial.snapshot?.cols ?? 80,
    rows: initial.snapshot?.rows ?? 24,
    data,
  };
}

async function open(target: TerminalTarget, seed?: "history" | "snapshot") {
  return TerminalAttachment.open(hubEndpoint(), {
    target,
    cursor: null,
    ...(seed ? { seed } : {}),
  });
}

export async function openSeed(target: TerminalTarget): Promise<{
  attachment: TerminalAttachment;
  seed: Seed;
}> {
  try {
    const attachment = await open(target, "history");
    const seed = combine(attachment.takeInitial());
    if (!seed) throw new Error("Terminal attachment seed is missing");
    return { attachment, seed };
  } catch (history) {
    if (!historyUnavailable(history)) throw history;
    try {
      const attachment = await open(target, "snapshot");
      const seed = combine(attachment.takeInitial());
      if (seed) return { attachment, seed };
      attachment.close();
    } catch (snapshot) {
      if (!historyUnavailable(snapshot)) throw snapshot;
    }
    const attachment = await open(target);
    const seed = combine(attachment.takeInitial());
    if (!seed) throw new Error("Terminal attachment seed is missing");
    return { attachment, seed };
  }
}
