// A minimal Chrome DevTools Protocol client over Node's built-in WebSocket, so
// the browser smoke test needs no npm dependencies.

export class Cdp {
  static async connect(webSocketUrl) {
    const socket = new WebSocket(webSocketUrl);
    await new Promise((resolve, reject) => {
      socket.addEventListener('open', resolve, { once: true });
      socket.addEventListener('error', () => reject(new Error(`cannot connect to ${webSocketUrl}`)), { once: true });
    });
    return new Cdp(socket);
  }

  constructor(socket) {
    this.socket = socket;
    this.nextId = 1;
    this.pending = new Map();
    this.handlers = [];
    socket.addEventListener('message', (event) => {
      const message = JSON.parse(event.data);
      if (message.id !== undefined) {
        const entry = this.pending.get(message.id);
        if (!entry) return;
        this.pending.delete(message.id);
        if (message.error) entry.reject(new Error(`${entry.method}: ${message.error.message}`));
        else entry.resolve(message.result);
      } else {
        for (const handler of this.handlers) handler(message);
      }
    });
    socket.addEventListener('close', () => {
      for (const { reject, method } of this.pending.values()) reject(new Error(`${method}: connection closed`));
      this.pending.clear();
    });
  }

  send(method, params = {}, sessionId) {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject, method });
      this.socket.send(JSON.stringify({ id, method, params, sessionId }));
    });
  }

  onEvent(handler) {
    this.handlers.push(handler);
  }

  /**
   * Attach to a target and start collecting what went wrong in it:
   * uncaught exceptions, console.error calls and error-level log entries.
   * @returns {Promise<Session>}
   */
  async attach(targetId) {
    const { sessionId } = await this.send('Target.attachToTarget', { targetId, flatten: true });
    const session = new Session(this, sessionId, targetId);
    await Promise.all([
      this.send('Runtime.enable', {}, sessionId),
      this.send('Log.enable', {}, sessionId).catch(() => {}),
    ]);
    return session;
  }

  close() {
    this.socket.close();
  }
}

export class Session {
  constructor(cdp, sessionId, targetId) {
    this.cdp = cdp;
    this.sessionId = sessionId;
    this.targetId = targetId;
    this.problems = [];
    cdp.onEvent((message) => {
      if (message.sessionId !== sessionId) return;
      const { method, params } = message;
      if (method === 'Runtime.exceptionThrown') {
        this.problems.push(`exception: ${params.exceptionDetails.exception?.description ?? params.exceptionDetails.text}`);
      } else if (method === 'Runtime.consoleAPICalled' && params.type === 'error') {
        this.problems.push(`console.error: ${params.args.map((a) => a.value ?? a.description).join(' ')}`);
      } else if (method === 'Log.entryAdded' && params.entry.level === 'error') {
        this.problems.push(`log: ${params.entry.text} ${params.entry.url ?? ''}`);
      }
    });
  }

  /** Evaluate JavaScript in the target and return its (JSON-serialisable) result. */
  async evaluate(expression) {
    const { result, exceptionDetails } = await this.cdp.send(
      'Runtime.evaluate',
      { expression, awaitPromise: true, returnByValue: true, userGesture: true },
      this.sessionId,
    );
    if (exceptionDetails) {
      throw new Error(`evaluate failed: ${exceptionDetails.exception?.description ?? exceptionDetails.text}\n${expression}`);
    }
    return result.value;
  }

  send(method, params) {
    return this.cdp.send(method, params, this.sessionId);
  }
}

export async function waitFor(predicate, { timeout = 10000, interval = 50, message = 'condition' } = {}) {
  const deadline = Date.now() + timeout;
  let last;
  while (Date.now() < deadline) {
    try {
      last = await predicate();
      if (last) return last;
    } catch (error) {
      last = error;
    }
    await new Promise((r) => setTimeout(r, interval));
  }
  throw new Error(`timed out waiting for ${message}${last instanceof Error ? `: ${last.message}` : ''}`);
}
