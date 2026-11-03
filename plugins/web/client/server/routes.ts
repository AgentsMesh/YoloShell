import { apiGet } from "./apiRoutes";
import { script, sharedScript, styles } from "./assets";
import { creationStats, startJob } from "./creationJobs";
import { editLayout } from "./layoutEdit";
import { poolStats } from "./hub";
import {
  authorized,
  body,
  headersFor,
  json,
  requestAllowed,
  type HttpContext,
} from "./http";
import { respond } from "./interaction";
import { runLifecycle } from "./lifecycle";
import { page } from "./page";
import { conversation } from "./read";
import { terminalActorCount } from "./terminalRegistry";
import { vendorAsset } from "./vendor";
import { prompt } from "./write";

export type RouteContext = HttpContext & { instance: string };

const MANIFEST = JSON.stringify({
  name: "Yolo Web",
  short_name: "Yolo",
  start_url: "/",
  display: "standalone",
  background_color: "#0d0b09",
  theme_color: "#0d0b09",
  icons: [{ src: "/icon.svg", sizes: "any", type: "image/svg+xml" }],
});
const ICON =
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="14" fill="#0d0b09"/><path d="M18 16l14 19 14-19h8L36 40v10h-8V40L10 16z" fill="#e8a33d"/></svg>';

export async function route(
  request: Request,
  context: RouteContext,
): Promise<Response> {
  const url = new URL(request.url);
  if (!requestAllowed(request, context)) {
    return headersFor(new Response("forbidden", { status: 403 }));
  }
  if (url.pathname === "/health") {
    return json({
      ok: true,
      instance: context.instance,
      pool: poolStats(),
      terminals: terminalActorCount(),
      creations: creationStats(),
    });
  }
  if (url.pathname === "/manifest.webmanifest") {
    return headersFor(
      new Response(MANIFEST, { headers: { "content-type": "application/manifest+json" } }),
    );
  }
  if (url.pathname === "/icon.svg") {
    return headersFor(new Response(ICON, { headers: { "content-type": "image/svg+xml" } }));
  }
  if (url.pathname === "/" && request.method === "GET") {
    return headersFor(new Response(page, { headers: { "content-type": "text/html; charset=utf-8" } }));
  }
  if (url.pathname === "/app.css" && request.method === "GET") {
    return headersFor(new Response(await styles(), { headers: { "content-type": "text/css; charset=utf-8" } }));
  }
  if (url.pathname.startsWith("/vendor/") && request.method === "GET") {
    const asset = await vendorAsset(url.pathname.slice("/vendor/".length));
    if (asset) return headersFor(asset);
  }
  const module = url.pathname.match(/^\/app\/([A-Za-z]+)\.js$/);
  if (module && request.method === "GET") {
    const source = await script(module[1]);
    if (source !== null) {
      return headersFor(new Response(source, { headers: { "content-type": "text/javascript; charset=utf-8" } }));
    }
  }
  const shared = url.pathname.match(/^\/shared\/([A-Za-z]+)\.js$/);
  if (shared && request.method === "GET") {
    const source = await sharedScript(shared[1]);
    if (source !== null) {
      return headersFor(new Response(source, { headers: { "content-type": "text/javascript; charset=utf-8" } }));
    }
  }
  if (request.method === "GET" && url.pathname.startsWith("/api/")) {
    const response = await apiGet(url, authorized(request, context));
    if (response) return headersFor(response);
  }
  if (request.method === "POST" && url.pathname.startsWith("/api/")) {
    if (!authorized(request, context)) {
      return headersFor(new Response("unauthorized", { status: 401 }));
    }
    const data = await body(request);
    switch (url.pathname) {
      case "/api/conversation":
        return json(await conversation(data));
      case "/api/prompt":
        return json(await prompt(data));
      case "/api/respond":
        return json(await respond(data));
      case "/api/shells":
        return json(startJob("shell", data), 202);
      case "/api/agents":
        return json(startJob("agent", data), 202);
      case "/api/lifecycle":
        return json(await runLifecycle(data));
      case "/api/layout":
        return json(await editLayout(data));
    }
  }
  return headersFor(new Response("not found", { status: 404 }));
}
