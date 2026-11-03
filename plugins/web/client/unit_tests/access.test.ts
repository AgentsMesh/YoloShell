import { expect, test } from "bun:test";
import { extraHosts, hostAllowed, originAllowed, parsedHost } from "../server/access";

const allowed = new Set(["127.0.0.1", "localhost", "192.0.2.10"]);

test("accepts only the local port and allowed host names", () => {
  expect(hostAllowed("127.0.0.1:4199", 4199, allowed)).toBe(true);
  expect(hostAllowed("localhost:4199", 4199, allowed)).toBe(true);
  expect(hostAllowed("LOCALHOST:4199", 4199, allowed)).toBe(true);
  expect(hostAllowed("192.0.2.10:4199", 4199, allowed)).toBe(true);
  expect(hostAllowed("192.0.2.10:4200", 4199, allowed)).toBe(false);
  expect(hostAllowed("evil.example:4199", 4199, allowed)).toBe(false);
  expect(hostAllowed(null, 4199, allowed)).toBe(false);
  expect(parsedHost("localhost")).toBeNull();
  expect(parsedHost("localhost:0")).toBeNull();
});

test("allows a missing origin but never a mismatched one", () => {
  expect(originAllowed(null, "192.0.2.10:4199")).toBe(true);
  expect(originAllowed("http://192.0.2.10:4199", "192.0.2.10:4199")).toBe(true);
  expect(originAllowed("http://evil.example", "192.0.2.10:4199")).toBe(false);
  expect(originAllowed("http://192.0.2.10:4199", null)).toBe(false);
});

test("accepts explicit ingress IPv4 hosts only", () => {
  expect(extraHosts("198.51.100.7, 192.0.2.10")).toEqual(["198.51.100.7", "192.0.2.10"]);
  expect(extraHosts(" 198.51.100.7 ")).toEqual(["198.51.100.7"]);
  expect(extraHosts("evil.example, 198.51.100.7:4199, 198.51.100.999")).toEqual([]);
  expect(extraHosts(undefined)).toEqual([]);
});
