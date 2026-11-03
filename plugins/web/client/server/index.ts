import { networkInterfaces } from "node:os";
import { extraHosts } from "./access";
import { readingToken } from "./auth";
import { acquireBrokerLock } from "./brokerLock";
import { recoverCreations } from "./creationJobs";
import { failure } from "./http";
import { route, type RouteContext } from "./routes";
import { TerminalHub } from "./websocket";

const port = Number(process.env.PORT ?? "4173");
const token = readingToken();
if (!token) throw new Error("plugin token is empty");
const instance = process.env.INSTANCE ?? "";
if (!instance) throw new Error("plugin instance is unavailable");
const brokerLock = acquireBrokerLock();
const allowedHosts = new Set(["127.0.0.1", "localhost"]);
for (const entries of Object.values(networkInterfaces())) {
  for (const entry of entries ?? []) {
    if (!entry.internal && entry.family === "IPv4") allowedHosts.add(entry.address);
  }
}
for (const host of extraHosts(process.env.YOLO_WEB_ALLOWED_HOSTS)) allowedHosts.add(host);
const context: RouteContext = { port, token, instance, allowedHosts };
const terminalHub = new TerminalHub(token, port, allowedHosts);
const server = Bun.serve({
  port,
  hostname: "0.0.0.0",
  // A slow Hub catalogue read must produce a JSON error, not a dropped socket.
  idleTimeout: 30,
  websocket: terminalHub.websocket,
  async fetch(request, server) {
    const upgrade = terminalHub.upgrade(request, server);
    if (upgrade !== null) return upgrade;
    try {
      return await route(request, context);
    } catch (error) {
      return failure(error);
    }
  },
});
let stopping = false;
for (const signal of ["SIGTERM", "SIGINT"] as const) {
  process.on(signal, () => {
    if (stopping) return;
    stopping = true;
    const force = setTimeout(() => process.exit(0), 3000);
    void terminalHub.shutdown().finally(() => {
      clearTimeout(force);
      server.stop();
      brokerLock.release();
      process.exit(0);
    });
  });
}

console.log(`yolo web client listening on 0.0.0.0:${port}`);
recoverCreations();
