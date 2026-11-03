import {
  closeSync,
  fsyncSync,
  linkSync,
  mkdirSync,
  openSync,
  readFileSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";

export type LockRecord = {
  pid: number;
  owner: string;
  instance: string;
  at: number;
};

const PARTIAL_GRACE_MS = 5_000;
const ATTEMPTS = 3;

function isAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return (error as { code?: string }).code === "EPERM";
  }
}

function parse(path: string): LockRecord | null {
  try {
    const value = JSON.parse(readFileSync(path, "utf8")) as Partial<LockRecord>;
    if (
      Number.isInteger(value.pid) &&
      (value.pid ?? 0) > 0 &&
      typeof value.owner === "string" &&
      typeof value.instance === "string" &&
      typeof value.at === "number"
    ) {
      return value as LockRecord;
    }
  } catch {
    // A partial or malformed lock is handled by the caller.
  }
  return null;
}

function writeTemporary(path: string, record: LockRecord): string {
  const temporary = `${path}.${record.owner}.tmp`;
  const fd = openSync(temporary, "w", 0o600);
  try {
    writeFileSync(fd, JSON.stringify(record));
    fsyncSync(fd);
  } finally {
    closeSync(fd);
  }
  return temporary;
}

function remove(path: string): void {
  try {
    unlinkSync(path);
  } catch {
    // Already gone or held by another process.
  }
}

/**
 * One broker owns one plugin data directory. The creation journal and the
 * local presence/controller projection are single-writer state; a second
 * process must fail closed instead of reading or replaying the same records.
 */
export class BrokerLock {
  private held = false;

  constructor(
    private readonly path: string | null,
    private readonly record: LockRecord,
    private readonly alive: (pid: number) => boolean = isAlive,
  ) {}

  acquire(): void {
    if (!this.path || this.held) return;
    mkdirSync(dirname(this.path), { recursive: true, mode: 0o700 });
    for (let attempt = 0; attempt < ATTEMPTS; attempt += 1) {
      const temporary = writeTemporary(this.path, this.record);
      try {
        linkSync(temporary, this.path);
        this.held = true;
        remove(temporary);
        return;
      } catch (error) {
        remove(temporary);
        if ((error as { code?: string }).code !== "EEXIST") throw error;
      }
      const existing = parse(this.path);
      if (existing) {
        if (existing.owner === this.record.owner) {
          this.held = true;
          return;
        }
        if (this.alive(existing.pid)) {
          throw new Error(
            `another web broker (pid ${existing.pid}) is using this data directory`,
          );
        }
        remove(this.path);
        continue;
      }
      let age = 0;
      try {
        age = Date.now() - statSync(this.path).mtimeMs;
      } catch {
        continue;
      }
      if (age > PARTIAL_GRACE_MS) {
        remove(this.path);
        continue;
      }
      throw new Error("another web broker is starting");
    }
    throw new Error("the web broker data-directory lock is unavailable");
  }

  release(): void {
    if (!this.path || !this.held) return;
    const existing = parse(this.path);
    if (existing?.owner === this.record.owner) remove(this.path);
    this.held = false;
  }
}

export function acquireBrokerLock(): BrokerLock {
  const data = process.env.YOLO_PLUGIN_DATA;
  const lock = new BrokerLock(data ? join(data, "broker.lock") : null, {
    pid: process.pid,
    owner: crypto.randomUUID(),
    instance: process.env.INSTANCE ?? "",
    at: Date.now(),
  });
  lock.acquire();
  return lock;
}
