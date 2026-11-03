import { HubWire, type HubEndpoint } from "./hubWire.ts";
import { collectPages } from "./pages.ts";
export { HubError } from "./hubError.ts";
export type { HubEndpoint } from "./hubWire.ts";

export class HubClient {
  private methods = new Set<string>();

  private constructor(
    private readonly wire: HubWire,
    private readonly destinationId: string,
  ) {}

  static async connect(endpoint: HubEndpoint): Promise<HubClient> {
    const wire = await HubWire.connect(endpoint);
    const client = new HubClient(wire, endpoint.hubId);
    try {
      const reply = (await client.call("hello", { destination: endpoint.hubId })) as {
        hello?: { hub_id?: string; methods?: string[] };
      };
      if (reply.hello?.hub_id !== endpoint.hubId) throw new Error("Hub identity mismatch");
      client.methods = new Set(reply.hello.methods ?? []);
    } catch (error) {
      client.close();
      throw error;
    }
    return client;
  }

  get hubId(): string {
    return this.destinationId;
  }

  supports(method: string): boolean {
    return this.methods.has(method);
  }

  get connected(): boolean { return this.wire.connected; }

  async request(
    method: string,
    params: Record<string, unknown>,
    destination = this.destination(),
    timeoutMs?: number,
  ): Promise<unknown> {
    return this.call(method, { destination, ...params }, timeoutMs);
  }

  async list(): Promise<unknown> {
    return {
      terminals: await this.catalogue("terminals", "list", { include_archived: false }),
      next: null,
    };
  }

  async listArchived(destination = this.destination()): Promise<unknown> {
    const rows = await this.catalogue(
      "terminals",
      "list",
      { include_archived: true },
      destination,
    );
    return {
      terminals: rows.filter(
        (row) => typeof row === "object" && row !== null && (row as { archived?: unknown }).archived === true,
      ),
      next: null,
    };
  }

  async agents(): Promise<unknown> {
    return { agents: await this.catalogue("agents", "list_agents"), next: null };
  }
  async listHubs(): Promise<unknown> {
    return { hubs: await this.catalogue("hubs", "list_hubs"), next: null };
  }
  async inventory(): Promise<unknown> {
    const [terminals, agents, hubs] = await Promise.all([this.list(), this.agents(), this.listHubs()]);
    return { terminals, agents, hubs };
  }
  async resolveConversation(binding: unknown, destination = this.destination()): Promise<unknown> {
    return this.call("resolve_conversation", { destination, binding });
  }
  async readConversation(request: unknown, destination = this.destination()): Promise<unknown> {
    return this.call("read_conversation", { destination, request });
  }
  async watchConversation(
    request: unknown,
    waitMs = 1000,
    destination = this.destination(),
  ): Promise<unknown> {
    return this.call("watch_conversation", { destination, request, wait_ms: waitMs });
  }
  async observeLease(target: unknown, destination = this.destination()): Promise<unknown> {
    return this.call("observe_lease", { destination, target });
  }
  async lease(request: unknown, destination = this.destination()): Promise<unknown> {
    return this.call("lease", { destination, request });
  }
  async renew(grant: unknown, destination = this.destination()): Promise<unknown> {
    return this.call("renew", { destination, grant });
  }
  async input(request: unknown, destination = this.destination()): Promise<unknown> {
    return this.call("input", { destination, request });
  }
  async release(grant: unknown, destination = this.destination()): Promise<unknown> {
    return this.call("release", { destination, grant });
  }
  async agentPrompt(request: unknown, destination = this.destination()): Promise<unknown> {
    return this.call("agent_prompt", { destination, request });
  }
  async agentControl(
    request: unknown,
    grant: unknown,
    destination = this.destination(),
  ): Promise<unknown> {
    return this.call("agent_control", { destination, request, grant });
  }
  async readTerminalHistory(
    request: unknown,
    destination = this.destination(),
  ): Promise<unknown> {
    return this.call("read_terminal_history", { destination, request });
  }
  async listLayouts(destination = this.destination()): Promise<unknown> {
    return this.call("list_layouts", { destination });
  }
  async getLayout(address: unknown, destination = this.destination()): Promise<unknown> {
    return this.call("get_layout", { destination, address });
  }
  async updateLayout(change: unknown, destination = this.destination()): Promise<unknown> {
    return this.call("update_layout", { destination, change });
  }

  /** Read every page of one Hub's catalogue aggregate. */
  private async catalogue(
    key: string,
    method: string,
    extra: Record<string, unknown> = {},
    destination = this.destination(),
  ): Promise<unknown[]> {
    return collectPages(key, async (after) => {
      const params = { destination, ...extra, after };
      return (await this.call(method, params)) as Record<string, unknown>;
    });
  }

  close(): void {
    this.wire.close();
  }

  private destination(): string {
    if (!this.destinationId) throw new Error("Hub destination is unavailable");
    return this.destinationId;
  }

  private call(method: string, params: unknown, timeoutMs?: number): Promise<unknown> {
    return this.wire.call(method, params, timeoutMs);
  }
}
