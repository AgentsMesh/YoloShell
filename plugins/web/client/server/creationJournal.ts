import {
  chmodSync,
  closeSync,
  fsyncSync,
  mkdirSync,
  openSync,
  readFileSync,
  readdirSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { ApiError } from "./errors";
import type { CreationIntent, IntentState } from "./creationIntent";

const STATES = new Set<IntentState>([
  "prepared",
  "dispatched",
  "unconfirmed",
  "settled",
  "failed",
]);

function isIntent(value: unknown): value is CreationIntent {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const item = value as Record<string, unknown>;
  return (
    item.version === 1 &&
    typeof item.id === "string" &&
    typeof item.idempotency_key === "string" &&
    typeof item.fingerprint === "string" &&
    (item.kind === "shell" || item.kind === "agent") &&
    typeof item.origin === "string" &&
    typeof item.request === "object" &&
    item.request !== null &&
    !Array.isArray(item.request) &&
    typeof item.state === "string" &&
    STATES.has(item.state as IntentState) &&
    typeof item.createdAt === "number" &&
    typeof item.updatedAt === "number"
  );
}

function privateDirectory(path: string): void {
  mkdirSync(path, { recursive: true, mode: 0o700 });
  try {
    chmodSync(path, 0o700);
  } catch {
    // The parent plugin data directory is already private.
  }
}

function syncDirectory(path: string): void {
  try {
    const fd = openSync(path, "r");
    try {
      fsyncSync(fd);
    } finally {
      closeSync(fd);
    }
  } catch {
    // Some filesystems do not support fsync on a directory.
  }
}

/** Read every valid record; malformed files are evidence to keep, not delete. */
export function readRecords(directory: string): CreationIntent[] {
  privateDirectory(directory);
  const records: CreationIntent[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (!entry.isFile() || !entry.name.endsWith(".json")) continue;
    try {
      const value = JSON.parse(readFileSync(join(directory, entry.name), "utf8"));
      if (!isIntent(value)) throw new Error("invalid intent record");
      records.push({ ...value, attempts: Number(value.attempts) || 0 });
    } catch (error) {
      console.error(
        `ignoring creation journal ${entry.name}:`,
        error instanceof Error ? error.message : error,
      );
    }
  }
  return records;
}

/** The record reaches stable storage before the Hub mutation is sent. */
export function writeRecord(directory: string, intent: CreationIntent): void {
  privateDirectory(directory);
  const path = join(directory, `${intent.id}.json`);
  const temporary = join(directory, `.${intent.id}.tmp`);
  let fd: number | null = null;
  try {
    fd = openSync(temporary, "w", 0o600);
    writeFileSync(fd, JSON.stringify(intent));
    fsyncSync(fd);
    closeSync(fd);
    fd = null;
    renameSync(temporary, path);
    syncDirectory(directory);
  } catch (error) {
    if (fd !== null) {
      try {
        closeSync(fd);
      } catch {
        // already closed
      }
    }
    try {
      unlinkSync(temporary);
    } catch {
      // no temporary record remains
    }
    throw new ApiError(
      503,
      "The creation request could not be recorded, so no session was started",
    );
  }
}

export function removeRecord(directory: string, id: string): void {
  try {
    unlinkSync(join(directory, `${id}.json`));
  } catch {
    // The in-memory projection is authoritative for this process.
  }
}
