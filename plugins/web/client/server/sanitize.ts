const PRIVATE_KEYS = new Set([
  "hub_id",
  "hubId",
  "socket",
  "token",
  "capability",
  "grant",
  "creation_token",
  "creation_capability",
  "provider_session_id",
  "native_session_reference",
]);

export function sanitize<T>(value: T): T {
  if (Array.isArray(value)) return value.map(sanitize) as T;
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .filter(([key]) => !PRIVATE_KEYS.has(key))
        .map(([key, item]) => [key, sanitize(item)]),
    ) as T;
  }
  return value;
}
