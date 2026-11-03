/**
 * Coalesces concurrent catalogue reads and reuses a recent result. Browser
 * polling must not multiply into overlapping Hub observation sweeps: one
 * in-flight load serves every caller, and only an expired entry starts the
 * next load.
 */
export class SnapshotCache<T> {
  private value: T | undefined;
  private loadedAt = Number.NEGATIVE_INFINITY;
  private loading: Promise<T> | null = null;
  private generation = 0;

  constructor(
    private readonly ttlMs: number,
    private readonly now: () => number = Date.now,
  ) {}

  async get(load: () => Promise<T>): Promise<T> {
    if (this.value !== undefined && this.now() - this.loadedAt < this.ttlMs) {
      return this.value;
    }
    if (this.loading) return this.loading;
    const generation = this.generation;
    let loading: Promise<T>;
    loading = load()
      .then((value) => {
        if (generation === this.generation) {
          this.value = value;
          this.loadedAt = this.now();
        }
        return value;
      })
      .finally(() => {
        if (this.loading === loading) this.loading = null;
      });
    this.loading = loading;
    return loading;
  }

  /** A write invalidates the entry and prevents an in-flight read from repopulating it. */
  clear(): void {
    this.generation += 1;
    this.value = undefined;
    this.loadedAt = Number.NEGATIVE_INFINITY;
    this.loading = null;
  }
}
