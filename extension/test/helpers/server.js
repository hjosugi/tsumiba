// Starts the real tsumiba (headless `serve` mode) in a throwaway home directory.
//
// The binary comes from $TSUMIBA_BIN, else $CARGO_TARGET_DIR/debug/tsumiba,
// else ../target/debug/tsumiba. Without one the integration tests are skipped
// with a message (CI builds the binary first and sets TSUMIBA_BIN).

import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '..', '..', '..');

export function tsumibaBinary() {
  const candidates = [
    process.env.TSUMIBA_BIN,
    process.env.CARGO_TARGET_DIR && join(process.env.CARGO_TARGET_DIR, 'debug', 'tsumiba'),
    join(repoRoot, 'target', 'debug', 'tsumiba'),
  ].filter(Boolean);
  return candidates.find((c) => existsSync(c)) ?? null;
}

export const SKIP_REASON =
  'no tsumiba binary found (set TSUMIBA_BIN, or run `cargo build -p tsumiba` first)';

/** `{ skip }` options for node:test: a reason string, or false when the binary exists. */
export const needsBinary = { skip: tsumibaBinary() ? false : SKIP_REASON };

/**
 * @param {{config?: string}} options  `config` is the text of config.toml.
 */
export async function startServer({ config } = {}) {
  const bin = tsumibaBinary();
  if (!bin) throw new Error(SKIP_REASON);
  const home = mkdtempSync(join(tmpdir(), 'tsumiba-ext-'));
  mkdirSync(join(home, 'config'), { recursive: true });
  if (config) writeFileSync(join(home, 'config', 'config.toml'), config);

  const env = {
    ...process.env,
    TSUMIBA_HOME: home,
    // `serve` has no window, but make sure nothing tries to open one.
    DISPLAY: '',
    WAYLAND_DISPLAY: '',
  };
  delete env.TSUMIBA_DATA_DIR;
  delete env.TSUMIBA_CONFIG_DIR;

  const child = spawn(bin, ['serve', '--port', '0'], { env, stdio: ['ignore', 'pipe', 'pipe'] });
  let stderr = '';
  child.stderr.on('data', (d) => (stderr += d));

  const port = await new Promise((resolvePort, reject) => {
    let out = '';
    const timer = setTimeout(() => reject(new Error(`tsumiba serve did not start:\n${out}${stderr}`)), 15000);
    child.stdout.on('data', (d) => {
      out += d;
      const m = /listening on 127\.0\.0\.1:(\d+)/.exec(out);
      if (m) {
        clearTimeout(timer);
        resolvePort(Number(m[1]));
      }
    });
    child.on('exit', (code) => {
      clearTimeout(timer);
      reject(new Error(`tsumiba serve exited early (${code}):\n${out}${stderr}`));
    });
  });

  const token = readFileSync(join(home, 'config', 'token'), 'utf8').trim();
  let stopped = false;

  const server = {
    port,
    token,
    home,
    bin,
    /** Run the CLI against this home (it talks to the running server). */
    cli(args, input) {
      const result = spawnSync(bin, args, { env, input, encoding: 'utf8' });
      return { status: result.status, stdout: result.stdout, stderr: result.stderr };
    },
    /** `{ notes, files }` as the CLI sees them. */
    list() {
      const result = server.cli(['list', '--json']);
      if (result.status !== 0) throw new Error(`tsumiba list failed: ${result.stderr}`);
      return JSON.parse(result.stdout);
    },
    async stop() {
      if (stopped) return;
      stopped = true;
      const exited = new Promise((r) => child.once('exit', r));
      server.cli(['quit']);
      const timer = setTimeout(() => child.kill('SIGKILL'), 5000);
      await exited;
      clearTimeout(timer);
    },
    async cleanup() {
      await server.stop();
      rmSync(home, { recursive: true, force: true });
    },
  };
  return server;
}
