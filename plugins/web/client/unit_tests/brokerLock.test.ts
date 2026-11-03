import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { BrokerLock, type LockRecord } from "../server/brokerLock";

function path(): string {
  return join(mkdtempSync(join(tmpdir(), "yolo-web-lock-")), "broker.lock");
}

function record(pid = 424242): LockRecord {
  return { pid, owner: crypto.randomUUID(), instance: "test", at: Date.now() };
}

describe("broker data-directory lock", () => {
  test("creates a private lock and refuses a second owner", () => {
    const file = path();
    const firstRecord = record();
    const first = new BrokerLock(file, firstRecord);
    first.acquire();
    expect(statSync(file).mode & 0o777).toBe(0o600);
    expect(JSON.parse(readFileSync(file, "utf8")).owner).toBe(firstRecord.owner);

    const second = new BrokerLock(file, record(), () => true);
    expect(() => second.acquire()).toThrow("another web broker");
    first.release();
    expect(() => statSync(file)).toThrow();
    const third = new BrokerLock(file, record());
    third.acquire();
    third.release();
  });

  test("replaces a lock held by a dead pid", () => {
    const file = path();
    const dead = record(999_999);
    writeFileSync(file, JSON.stringify(dead));
    const lock = new BrokerLock(file, record(), () => false);
    lock.acquire();
    expect(JSON.parse(readFileSync(file, "utf8")).owner).not.toBe(dead.owner);
    lock.release();
  });

  test("removes an old malformed lock but respects a fresh one", () => {
    const file = path();
    writeFileSync(file, "not-json");
    const fresh = new BrokerLock(file, record(), () => true);
    expect(() => fresh.acquire()).toThrow("starting");

    const old = new Date(Date.now() - 10_000);
    utimesSync(file, old, old);
    const lock = new BrokerLock(file, record(), () => true);
    lock.acquire();
    lock.release();
  });
});
