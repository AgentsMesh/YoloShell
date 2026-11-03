import { readdir } from "node:fs/promises";

const BROWSER = new URL("../browser/", import.meta.url);
const SHARED = new URL("../shared/", import.meta.url);
const transpiler = new Bun.Transpiler({ loader: "ts" });
const scripts = new Map<string, string>();
const sharedScripts = new Map<string, string>();
let modules: Promise<Set<string>> | null = null;
let sharedModules: Promise<Set<string>> | null = null;
let css: string | null = null;

async function load(
  directory: URL,
  cache: Map<string, string>,
  names: Promise<Set<string>>,
  name: string,
): Promise<string | null> {
  if (!(await names).has(name)) return null;
  const cached = cache.get(name);
  if (cached) return cached;
  const file = Bun.file(new URL(`${name}.ts`, directory));
  if (!(await file.exists())) return null;
  const value = transpiler.transformSync(await file.text());
  cache.set(name, value);
  return value;
}

export function script(name: string): Promise<string | null> {
  return load(BROWSER, scripts, moduleNames(), name);
}

export function sharedScript(name: string): Promise<string | null> {
  return load(SHARED, sharedScripts, sharedModuleNames(), name);
}

export async function styles(): Promise<string> {
  if (css === null) {
    css = await Bun.file(new URL("./styles.css", import.meta.url)).text();
  }
  return css;
}

function list(directory: URL): Promise<Set<string>> {
  return readdir(directory).then(
    (names) =>
      new Set(
        names
          .filter((name) => name.endsWith(".ts"))
          .map((name) => name.slice(0, -".ts".length)),
      ),
  );
}

function moduleNames(): Promise<Set<string>> {
  modules ??= list(BROWSER);
  return modules;
}

function sharedModuleNames(): Promise<Set<string>> {
  sharedModules ??= list(SHARED);
  return sharedModules;
}
