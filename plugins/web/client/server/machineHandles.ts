import { createHmac, randomUUID } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ApiError } from "./errors";
import { hubId } from "./hub";

const handles = new Map<string, string>();
const hubs = new Map<string, string>();
const labels = new Map<string, string>();
let nextPeer = 2;
let secret: Buffer | null | undefined;

function key(): Buffer | null {
  if (secret !== undefined) return secret;
  const data = process.env.YOLO_PLUGIN_DATA;
  if (!data) {
    secret = null;
    return secret;
  }
  try {
    const path = join(data, "machine-key");
    if (existsSync(path)) {
      secret = readFileSync(path);
    } else {
      mkdirSync(data, { recursive: true, mode: 0o700 });
      const created = Buffer.from(randomUUID().replaceAll("-", ""), "hex");
      writeFileSync(path, created, { mode: 0o600 });
      secret = created;
    }
    if (secret.byteLength !== 16) secret = null;
  } catch {
    secret = null;
  }
  return secret;
}

function stableHandle(hub: string): string {
  const raw = key();
  if (!raw) return randomUUID();
  const bytes = createHmac("sha256", raw).update(hub).digest().subarray(0, 16);
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = bytes.toString("hex");
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20),
  ].join("-");
}

/** A stable opaque browser handle for one Hub; raw Hub ids stay server-side. */
export function machineFor(hub: string): string {
  let handle = handles.get(hub);
  if (!handle) {
    handle = stableHandle(hub);
    handles.set(hub, handle);
    hubs.set(handle, hub);
  }
  return handle;
}

export function hubFor(machine: string): string {
  const hub = hubs.get(machine);
  if (!hub) throw new ApiError(400, "Unknown machine");
  return hub;
}

export function entryMachine(): string {
  return machineFor(hubId());
}

/** Configured peer labels win; otherwise the entry is local and peers are numbered. */
export function labelFor(hub: string, configured?: string): string {
  if (configured?.trim()) {
    labels.set(hub, configured.trim());
    return configured.trim();
  }
  const known = labels.get(hub);
  if (known) return known;
  const label = hub === hubId() ? "This machine" : `Machine ${nextPeer++}`;
  labels.set(hub, label);
  return label;
}
