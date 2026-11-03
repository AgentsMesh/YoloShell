import { isIPv4 } from "node:net";

export type ParsedHost = { name: string; port: number };

export function extraHosts(value: string | undefined): string[] {
  return (value ?? "")
    .split(",")
    .map((entry) => entry.trim().toLowerCase())
    .filter((entry) => isIPv4(entry));
}

export function parsedHost(host: string | null): ParsedHost | null {
  if (!host) return null;
  const separator = host.lastIndexOf(":");
  if (separator <= 0) return null;
  const name = host.slice(0, separator).toLowerCase();
  const port = Number(host.slice(separator + 1));
  if (!Number.isInteger(port) || port <= 0 || port > 65535) return null;
  return { name, port };
}

export function hostAllowed(host: string | null, port: number, allowed: Set<string>): boolean {
  const parsed = parsedHost(host);
  return parsed !== null && parsed.port === port && allowed.has(parsed.name);
}

export function originAllowed(origin: string | null, host: string | null): boolean {
  return !origin || (host !== null && origin === `http://${host}`);
}
