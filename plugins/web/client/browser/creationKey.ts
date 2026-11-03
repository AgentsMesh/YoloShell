import { ApiError } from "./api.js";
import { CreationJobError } from "./creationJob.js";

const KEY = "yolo-creation-request";
const MAX_AGE_MS = 24 * 60 * 60_000;

type Stored = { signature: string; key: string; at: number };

function signature(mode: string, body: Record<string, unknown>): string {
  return JSON.stringify([
    mode,
    body.machine ?? "",
    body.cwd ?? "",
    body.name ?? "",
    body.provider ?? "",
  ]);
}

function randomId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  return [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

function read(): Stored | null {
  try {
    const value = JSON.parse(sessionStorage.getItem(KEY) ?? "null");
    if (
      typeof value?.signature === "string" &&
      typeof value?.key === "string" &&
      typeof value?.at === "number"
    ) {
      return value;
    }
  } catch {
    // private mode or malformed storage
  }
  return null;
}

/** Reuse one intent key for an identical retry; a changed form begins a new intent. */
export function intentKey(mode: string, body: Record<string, unknown>): string {
  const expected = signature(mode, body);
  const stored = read();
  if (stored && stored.signature === expected && Date.now() - stored.at < MAX_AGE_MS) {
    return stored.key;
  }
  const key = randomId();
  try {
    sessionStorage.setItem(KEY, JSON.stringify({ signature: expected, key, at: Date.now() }));
  } catch {
    // the key still works for this submission
  }
  return key;
}

export function forgetIntentKey(key: string): void {
  try {
    if (read()?.key === key) sessionStorage.removeItem(KEY);
  } catch {
    // nothing to clear
  }
}

/** Only a confirmed rejection may abandon the key; uncertain outcomes keep it. */
export function keepIntentKey(error: unknown): boolean {
  if (error instanceof CreationJobError) return error.retryable;
  if (error instanceof ApiError) {
    return error.status === 401 || error.status === 403 || error.status >= 500;
  }
  return true;
}
