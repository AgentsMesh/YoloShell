import { HubError } from "../../sdk/src/hub.ts";
import { hostAllowed, originAllowed } from "./access";
import { sameToken } from "./auth";
import { ApiError } from "./errors";
import { sanitize } from "./sanitize";
import { record } from "./validate";

export type HttpContext = {
  port: number;
  token: string;
  allowedHosts: Set<string>;
};

export function headersFor(response: Response): Response {
  response.headers.set("cache-control", "no-store");
  response.headers.set("x-content-type-options", "nosniff");
  response.headers.set("x-frame-options", "DENY");
  response.headers.set("referrer-policy", "no-referrer");
  response.headers.set(
    "content-security-policy",
    "default-src 'none'; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self'; img-src 'self'; manifest-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
  );
  return response;
}

export function json(value: unknown, status = 200): Response {
  return headersFor(Response.json(sanitize(value), { status }));
}

export function requestAllowed(request: Request, context: HttpContext): boolean {
  const host = request.headers.get("host");
  return (
    hostAllowed(host, context.port, context.allowedHosts) &&
    originAllowed(request.headers.get("origin"), host)
  );
}

export function authorized(request: Request, context: HttpContext): boolean {
  const value = request.headers.get("authorization") ?? "";
  return value.startsWith("Bearer ") && sameToken(value.slice(7), context.token);
}

export async function body(request: Request): Promise<Record<string, unknown>> {
  const type = request.headers.get("content-type") ?? "";
  if (!type.toLowerCase().startsWith("application/json")) {
    throw new ApiError(415, "JSON body required");
  }
  const text = await request.text();
  if (text.length > 262_144) throw new ApiError(413, "Request is too large");
  try {
    return record(JSON.parse(text));
  } catch (error) {
    if (error instanceof ApiError) throw error;
    throw new ApiError(400, "Invalid JSON");
  }
}

export function failure(error: unknown): Response {
  if (error instanceof ApiError) return json({ error: error.message }, error.status);
  if (error instanceof HubError) {
    if (error.code === -32602) return json({ error: "Hub rejected the request" }, 400);
    const data = error.data as { code?: unknown } | null;
    if (data?.code === "conflict") {
      return json({ error: "Terminal is controlled by another client" }, 409);
    }
    return json({ error: "Hub request failed" }, 502);
  }
  console.error(error);
  return json({ error: "Request failed" }, 502);
}
