/** Minimal Chrome DevTools client shared by the browser acceptance scenarios. */

export class Cdp {
  readonly errors: string[] = [];
  private readonly pending = new Map<number, { resolve: (value: any) => void; reject: (error: Error) => void }>();
  private sequence = 0;

  private constructor(private readonly ws: WebSocket) {}

  static async connect(url: string, enableRuntime = true): Promise<Cdp> {
    const ws = new WebSocket(url);
    const client = new Cdp(ws);
    ws.onmessage = (event) => client.receive(event);
    await new Promise<void>((resolve, reject) => {
      ws.onopen = () => resolve();
      ws.onerror = () => reject(new Error(`CDP connection failed: ${url}`));
    });
    if (enableRuntime) {
      await client.call("Runtime.enable");
      await client.call("Log.enable");
    }
    return client;
  }

  call(method: string, params: Record<string, unknown> = {}): Promise<any> {
    return new Promise((resolve, reject) => {
      const id = ++this.sequence;
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  async evaluate(expression: string): Promise<any> {
    const result = await this.call("Runtime.evaluate", {
      expression,
      returnByValue: true,
      awaitPromise: true,
    });
    if (result.exceptionDetails) throw new Error(result.exceptionDetails.text ?? "evaluation failed");
    return result.result?.value;
  }

  async waitFor(expression: string, timeoutMs = 8_000): Promise<void> {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      if (await this.evaluate(expression)) return;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error(`timed out waiting for ${expression}`);
  }

  async click(selector: string): Promise<void> {
    await this.evaluate(`document.querySelector(${JSON.stringify(selector)})?.click()`);
  }

  async fill(selector: string, value: string): Promise<void> {
    await this.evaluate(
      `(()=>{const e=document.querySelector(${JSON.stringify(selector)}); e.value=${JSON.stringify(value)}; return true})()`,
    );
  }

  async wheel(selector: string, deltaY: number): Promise<void> {
    const rect = JSON.parse(
      await this.evaluate(
        `(()=>{const e=document.querySelector(${JSON.stringify(selector)}); const r=e.getBoundingClientRect(); return JSON.stringify({x:r.x+r.width/2,y:r.y+r.height/2})})()`,
      ),
    );
    await this.call("Input.dispatchMouseEvent", {
      type: "mouseWheel",
      x: rect.x,
      y: rect.y,
      deltaX: 0,
      deltaY,
    });
  }

  async type(text: string): Promise<void> {
    await this.call("Input.insertText", { text });
  }

  async enter(): Promise<void> {
    const key = {
      key: "Enter",
      code: "Enter",
      windowsVirtualKeyCode: 13,
      nativeVirtualKeyCode: 13,
      text: "\r",
      unmodifiedText: "\r",
    };
    await this.call("Input.dispatchKeyEvent", { type: "keyDown", ...key });
    await this.call("Input.dispatchKeyEvent", { type: "keyUp", ...key });
  }

  close(): void {
    this.ws.close();
  }

  private receive(event: MessageEvent): void {
    const message = JSON.parse(String(event.data));
    const waiter = this.pending.get(message.id);
    if (waiter) {
      this.pending.delete(message.id);
      if (message.error) waiter.reject(new Error(JSON.stringify(message.error)));
      else waiter.resolve(message.result);
      return;
    }
    if (message.method === "Runtime.exceptionThrown") {
      this.errors.push(String(message.params.exceptionDetails?.text ?? "exception"));
    }
    if (message.method === "Runtime.consoleAPICalled" && message.params.type === "error") {
      this.errors.push(
        message.params.args.map((arg: any) => arg.value ?? arg.description).join(" "),
      );
    }
  }
}

export function assert(condition: boolean, message: string): void {
  if (!condition) throw new Error(message);
}

export async function unlock(page: Cdp, token?: string): Promise<void> {
  if (await page.evaluate("document.getElementById('gate')?.hidden !== false")) return;
  assert(Boolean(token), "the page is gated; set YOLO_WEB_E2E_TOKEN");
  await page.evaluate(
    `(()=>{const input=document.getElementById('token'); input.value=${JSON.stringify(token)}; document.getElementById('gate-form').requestSubmit(); return true})()`,
  );
  await page.waitFor("document.getElementById('gate')?.hidden === true", 10_000);
}

export async function reload(page: Cdp): Promise<void> {
  await page.call("Page.enable");
  await page.call("Page.reload", { ignoreCache: true });
  await page.waitFor(
    "document.readyState==='complete' && !!document.getElementById('app')",
    10_000,
  );
}
