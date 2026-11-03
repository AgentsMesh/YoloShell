import { createHash } from "node:crypto";
import { ApiError } from "./errors";
import { hubId } from "./hub";
import { hubFor } from "./machineHandles";
import { AGENT_SCHEMA } from "./schema";
import { record } from "./validate";

export type Kind = "shell" | "agent";
export type Spec = {
  kind: Kind;
  origin: string;
  cwd: string;
  name?: string;
  provider?: string;
};
export type Prepared = {
  kind: Kind;
  origin: string;
  provider?: string;
  request: Record<string, unknown>;
};

const TERMINAL_SCHEMA = 1;
const DEFAULT_SIZE = { cols: 120, rows: 32 };
const PROVIDERS = new Set(["codex", "claude_code", "qwen_code", "gemini_cli", "grok"]);
const KEY_PATTERN = /^[A-Za-z0-9_-]{16,128}$/;

function text(value: unknown, label: string, maximum: number): string {
  if (typeof value !== "string") throw new ApiError(400, `${label} is required`);
  const text = value.trim();
  if (!text || text.length > maximum || text.includes("\0")) {
    throw new ApiError(400, `Invalid ${label.toLowerCase()}`);
  }
  return text;
}

function optionalText(value: unknown, label: string, maximum: number): string | undefined {
  if (value === undefined || value === null || value === "") return undefined;
  return text(value, label, maximum);
}

function origin(item: Record<string, unknown>): string {
  const machine = item.machine;
  if (machine === undefined || machine === null) return hubId();
  if (typeof machine !== "string") throw new ApiError(400, "Invalid machine");
  return hubFor(machine);
}

export function spec(kind: Kind, body: Record<string, unknown>): Spec {
  const item = record(body);
  const cwd = text(item.cwd, "Directory", 4096);
  const name = optionalText(item.name, "Name", 128);
  if (kind === "shell") {
    return {
      kind,
      origin: origin(item),
      cwd,
      ...(name ? { name } : {}),
    };
  }
  const provider = text(item.provider, "Provider", 64);
  if (!PROVIDERS.has(provider)) throw new ApiError(400, "Unknown agent provider");
  return {
    kind,
    origin: origin(item),
    cwd,
    provider,
    ...(name ? { name } : {}),
  };
}

/** The full original request is the only recovery authority for a shell. */
export function prepare(value: Spec): Prepared {
  const base = {
    schema_version: value.kind === "shell" ? TERMINAL_SCHEMA : AGENT_SCHEMA,
    request_id: crypto.randomUUID(),
    ...(value.name ? { name: value.name } : {}),
    cwd: value.cwd,
    size: DEFAULT_SIZE,
  };
  const request =
    value.kind === "shell"
      ? { ...base, creation_token: crypto.randomUUID() }
      : {
          ...base,
          start_token: crypto.randomUUID(),
          provider_id: value.provider,
        };
  return {
    kind: value.kind,
    origin: value.origin,
    ...(value.provider ? { provider: value.provider } : {}),
    request,
  };
}

function comparable(value: unknown): string {
  if (value === undefined || value === null) return "";
  return typeof value === "string" ? value.trim() : JSON.stringify(value);
}

/** Compare normalized retries without resolving the machine mapping. */
export function fingerprint(kind: Kind, body: Record<string, unknown>): string {
  const item = record(body);
  return createHash("sha256")
    .update(
      JSON.stringify([
        kind,
        comparable(item.machine),
        comparable(item.cwd),
        comparable(item.name),
        comparable(item.provider),
      ]),
    )
    .digest("hex");
}

/** A browser retry reuses its key; an API client without one gets a fresh intent. */
export function idempotencyKey(body: Record<string, unknown>): string {
  const value = body.idempotency_key;
  if (value === undefined || value === null || value === "") return crypto.randomUUID();
  if (typeof value !== "string" || !KEY_PATTERN.test(value)) {
    throw new ApiError(400, "Invalid idempotency key");
  }
  return value;
}
