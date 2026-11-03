export const HEARTBEAT_PING_MS = 15_000;
export const HEARTBEAT_STALE_MS = 45_000;
export const HEARTBEAT_PROBE_MS = 4_000;

export const HEARTBEAT_PING = '{"type":"ping"}';

/** A socket with no frame for this long is half-open or gone. */
export function heartbeatStale(lastSeen: number, now: number): boolean {
  return now - lastSeen > HEARTBEAT_STALE_MS;
}
