import { chmodSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { networkInterfaces } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

type State = { pid: number; port: number; instance: string };

const root = fileURLToPath(new URL("..", import.meta.url));
const data = process.env.YOLO_PLUGIN_DATA ?? join(root, ".data");
const statePath = join(data, "state.json");
const tokenPath = join(data, "token");
const portStart = Number(process.env.YOLO_PLUGIN_PORT ?? "4173");

function state(): State | null {
  try {
    const value = JSON.parse(readFileSync(statePath, "utf8")) as Partial<State>;
    if (
      Number.isInteger(value.pid) && value.pid! > 1 &&
      Number.isInteger(value.port) && value.port! > 0 && value.port! < 65536 &&
      typeof value.instance === "string" && value.instance.length === 32
    ) {
      return { pid: value.pid!, port: value.port!, instance: value.instance };
    }
  } catch {
    // absent or malformed state is treated as stopped
  }
  return null;
}

function alive(value: State | null): value is State {
  if (!value) return false;
  try {
    process.kill(value.pid, 0);
    return true;
  } catch {
    return false;
  }
}

async function health(value: State): Promise<boolean> {
  try {
    const response = await fetch(`http://127.0.0.1:${value.port}/health`, {
      signal: AbortSignal.timeout(1000),
    });
    if (!response.ok) return false;
    const body = (await response.json()) as { instance?: unknown };
    return body.instance === value.instance;
  } catch {
    return false;
  }
}

function lanUrls(port: number): string[] {
  const urls: string[] = [];
  for (const entries of Object.values(networkInterfaces())) {
    for (const entry of entries ?? []) {
      if (!entry.internal && entry.family === "IPv4") urls.push(`http://${entry.address}:${port}`);
    }
  }
  return urls;
}

async function ready(value: State): Promise<boolean> {
  const deadline = Date.now() + 4000;
  while (Date.now() < deadline) {
    if (await health(value)) return true;
    await Bun.sleep(50);
  }
  return false;
}

async function start() {
  const current = state();
  if (current && alive(current) && (await health(current))) {
    console.log(`already running at http://127.0.0.1:${current.port}`);
    for (const url of lanUrls(current.port)) console.log(`LAN: ${url}`);
    return;
  }
  mkdirSync(data, { recursive: true, mode: 0o700 });
  chmodSync(data, 0o700);
  const token = [...crypto.getRandomValues(new Uint8Array(32))]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
  const instance = [...crypto.getRandomValues(new Uint8Array(16))]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
  writeFileSync(tokenPath, token, { mode: 0o600 });
  chmodSync(tokenPath, 0o600);
  let selected: number | null = null;
  for (let candidate = portStart; candidate < portStart + 20; candidate += 1) {
    try {
      const probe = Bun.serve({ port: candidate, hostname: "0.0.0.0", fetch: () => new Response("probe") });
      await probe.stop();
      selected = candidate;
      break;
    } catch {
      // try the next loopback port
    }
  }
  if (selected === null) throw new Error("no loopback port available");
  const child = Bun.spawn({
    cmd: ["bun", join(root, "server/index.ts")],
    cwd: root,
    env: {
      ...process.env,
      PORT: String(selected),
      INSTANCE: instance,
      YOLO_PLUGIN_DATA: data,
      YOLO_HUB_SOCKET: process.env.YOLO_HUB_SOCKET ?? "",
      YOLO_HUB_ID: process.env.YOLO_HUB_ID ?? "",
    },
    stdin: "ignore",
    stdout: "ignore",
    stderr: "ignore",
  });
  child.unref?.();
  const next = { pid: child.pid, port: selected, instance };
  writeFileSync(statePath, JSON.stringify(next), { mode: 0o600 });
  chmodSync(statePath, 0o600);
  if (!(await ready(next))) {
    try {
      process.kill(next.pid, "SIGTERM");
    } catch {
      // the child already exited
    }
    rmSync(statePath);
    throw new Error("web server did not become ready");
  }
  console.log(`started at http://127.0.0.1:${selected}`);
  for (const url of lanUrls(selected)) console.log(`LAN: ${url}`);
  console.log("LAN access enabled (HTTP, token required)");
  console.log(`token file: ${tokenPath}`);
}

async function status() {
  const current = state();
  if (current && alive(current) && (await health(current))) {
    console.log(`running at http://127.0.0.1:${current.port}`);
    return;
  }
  console.log("stopped");
  process.exitCode = 1;
}

async function stop() {
  const current = state();
  if (current && alive(current) && (await health(current))) {
    try {
      process.kill(current.pid, "SIGTERM");
    } catch {
      // the daemon exited between the health check and the signal
    }
    console.log("stopped");
  } else if (current) {
    console.log("stale state removed");
  } else {
    console.log("stopped");
  }
  if (existsSync(statePath)) rmSync(statePath);
}

const command = process.argv[2];
if (command === "start") await start();
else if (command === "status") await status();
else if (command === "stop") await stop();
else {
  console.error("usage: plugin.ts start|status|stop");
  process.exitCode = 2;
}
