import { api, ApiError } from "./api.js";

const TIMEOUT_MS = 90_000;
const UNKNOWN_GRACE_MS = 15_000;
const POLL_MS = 300;

/** A rejected vs unconfirmed outcome decides whether the retry key may be reused. */
export class CreationJobError extends Error {
  constructor(
    message: string,
    readonly retryable: boolean,
  ) {
    super(message);
    this.name = "CreationJobError";
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Poll a server-owned intent; the journal survives this tab and plugin restarts. */
export async function waitForJob(id: string): Promise<any> {
  const deadline = Date.now() + TIMEOUT_MS;
  let unknownSince: number | null = null;
  while (Date.now() < deadline) {
    try {
      const job = (await api.job(id)) as {
        state?: string;
        result?: any;
        error?: string;
      };
      if (job.state === "done") return job.result;
      if (job.state === "error") {
        throw new CreationJobError(job.error || "Creation failed", false);
      }
      if (job.state === "unknown") {
        unknownSince ??= Date.now();
        if (Date.now() - unknownSince >= UNKNOWN_GRACE_MS) {
          throw new CreationJobError(
            job.error ||
              "The machine has not confirmed this session. Check Agents and Shells before retrying.",
            true,
          );
        }
      }
    } catch (error) {
      if (error instanceof CreationJobError) throw error;
      if (error instanceof ApiError) {
        if (error.status === 401) throw error;
        if (error.status === 404) {
          throw new CreationJobError(
            "The plugin cannot find this creation. Check Agents and Shells before retrying.",
            true,
          );
        }
      }
    }
    await sleep(POLL_MS);
  }
  throw new CreationJobError(
    "Creation is still starting; check Agents and Shells before retrying.",
    true,
  );
}
