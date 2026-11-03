/** Page cursors are opaque protocol values; only the owning Hub interprets them. */
export type PageCursor = unknown;

const PAGE_LIMIT = 64;

/**
 * Collect every row of a paged catalogue reply. The first read uses `null` and
 * every later read receives the previous reply's `next` cursor unchanged. A
 * repeated cursor ends the walk instead of looping forever, and `PAGE_LIMIT`
 * bounds a server that never reports the end.
 */
export async function collectPages(
  key: string,
  read: (after: PageCursor) => Promise<Record<string, unknown>>,
): Promise<unknown[]> {
  const rows: unknown[] = [];
  let after: PageCursor = null;
  for (let page = 0; page < PAGE_LIMIT; page++) {
    const reply = await read(after);
    const batch = reply[key];
    if (Array.isArray(batch)) rows.push(...batch);
    const next = reply.next ?? null;
    if (next === null) return rows;
    if (JSON.stringify(next) === JSON.stringify(after)) return rows;
    after = next;
  }
  return rows;
}
