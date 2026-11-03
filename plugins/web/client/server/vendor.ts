import { createHash } from "node:crypto";

type VendorManifest = {
  packages: Array<{ files: Record<string, string> }>;
};

const DIRECTORY = new URL("../vendor/", import.meta.url);
const TYPES: Record<string, string> = {
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
};
let assets: Promise<Map<string, { data: Uint8Array; type: string }>> | null = null;

export async function vendorAsset(name: string): Promise<Response | null> {
  const loaded = await load();
  const entry = loaded.get(name);
  if (!entry) return null;
  return new Response(entry.data, { headers: { "content-type": entry.type } });
}

async function load(): Promise<Map<string, { data: Uint8Array; type: string }>> {
  assets ??= verify();
  return assets;
}

/** Verifies every manifest entry and caches only the servable assets. */
async function verify(): Promise<Map<string, { data: Uint8Array; type: string }>> {
  const manifest = (await Bun.file(new URL("manifest.json", DIRECTORY)).json()) as VendorManifest;
  const files = manifest.packages.flatMap((pack) => Object.entries(pack.files));
  const routes = new Map<string, { data: Uint8Array; type: string }>();
  for (const [name, expected] of files) {
    if (!name || name !== name.split(/[\\/]/).pop()) {
      throw new Error(`Vendored asset name is invalid: ${name}`);
    }
    const file = Bun.file(new URL(name, DIRECTORY));
    if (!(await file.exists())) throw new Error(`Vendored asset is missing: ${name}`);
    const data = new Uint8Array(await file.arrayBuffer());
    const actual = createHash("sha256").update(data).digest("hex");
    if (actual !== expected) throw new Error(`Vendored asset hash mismatch: ${name}`);
    const extension = name.slice(name.lastIndexOf("."));
    const type = TYPES[extension];
    if (type) routes.set(name, { data, type });
  }
  return routes;
}
