import type { ConversationResult } from "./types.js";

const TOKEN_KEY = "yolo-token";
const REQUEST_TIMEOUT_MS = 20_000;
const MUTATION_TIMEOUT_MS = 60_000;
let token = "";

export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

export function loadToken(): string {
  try {
    token = sessionStorage.getItem(TOKEN_KEY) ?? "";
  } catch {
    token = "";
  }
  return token;
}

export function saveToken(value: string): string {
  token = value.trim();
  try {
    sessionStorage.setItem(TOKEN_KEY, token);
  } catch {
    // session storage may be unavailable in private mode
  }
  return token;
}

export function forgetToken(): void {
  token = "";
  try {
    sessionStorage.removeItem(TOKEN_KEY);
  } catch {
    // nothing to clear
  }
}

export function hasToken(): boolean {
  return token.length > 0;
}

async function failure(response: Response): Promise<ApiError> {
  if (response.status === 401) return new ApiError(401, "Token rejected");
  let message = `Request failed (${response.status})`;
  try {
    const value = await response.json();
    if (typeof value?.error === "string") message = value.error;
  } catch {
    // keep the status message
  }
  return new ApiError(response.status, message);
}

async function request(path: string, body?: unknown, timeout = REQUEST_TIMEOUT_MS): Promise<any> {
  const headers: Record<string, string> = { authorization: `Bearer ${token}` };
  if (body !== undefined) headers["content-type"] = "application/json";
  const response = await fetch(path, {
    method: body === undefined ? "GET" : "POST",
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(timeout),
  });
  if (!response.ok) throw await failure(response);
  return response.json();
}

async function binary(
  path: string,
  truncatedHeader = "x-archive-truncated",
): Promise<{ data: Uint8Array; truncated: boolean }> {
  const response = await fetch(path, {
    headers: { authorization: `Bearer ${token}` },
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  if (!response.ok) throw await failure(response);
  return {
    data: new Uint8Array(await response.arrayBuffer()),
    truncated: response.headers.get(truncatedHeader) === "true",
  };
}

export const api = {
  inventory: () => request("/api/inventory"),
  conversation: (body: unknown): Promise<ConversationResult> =>
    request("/api/conversation", body),
  respond: (body: unknown): Promise<any> => request("/api/respond", body),
  prompt: (body: unknown) => request("/api/prompt", body),
  createShell: (body: unknown) => request("/api/shells", body, MUTATION_TIMEOUT_MS),
  startAgent: (body: unknown) => request("/api/agents", body, MUTATION_TIMEOUT_MS),
  job: (id: string) => request(`/api/jobs/${id}`),
  lifecycle: (body: unknown) => request("/api/lifecycle", body, MUTATION_TIMEOUT_MS),
  archive: (machine?: string) =>
    request(`/api/archive${machine ? `?machine=${encodeURIComponent(machine)}` : ""}`),
  archiveHistory: (query: URLSearchParams) => binary(`/api/archive/history?${query}`),
  terminalLog: (query: URLSearchParams) =>
    binary(`/api/terminal/log?${query}`, "x-log-truncated"),
};
