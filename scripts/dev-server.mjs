import { spawn } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:net";

const host = "127.0.0.1";
const basePort = Number(process.env.BW_DEV_PORT ?? 1420);
const port = await findOpenPort(basePort);
const devUrl = `http://${host}:${port}`;
const originalConfig = readTauriConfig();
const originalDevUrl = originalConfig.config.build.devUrl;

updateTauriDevUrl(devUrl);

const child = spawn(
  "vite",
  ["--host", host, "--port", String(port)],
  {
    stdio: "inherit",
    shell: process.platform === "win32",
    env: {
      ...process.env,
      BW_DEV_URL: devUrl,
    },
  },
);

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => {
    restoreTauriDevUrl(originalDevUrl);
    child.kill(signal);
  });
}

child.on("exit", (code, signal) => {
  restoreTauriDevUrl(originalDevUrl);
  if (signal) process.kill(process.pid, signal);
  process.exit(code ?? 0);
});

async function findOpenPort(start) {
  for (let port = start; port < start + 40; port += 1) {
    if (await canListen(port)) return port;
  }
  throw new Error(`No open dev port found in range ${start}-${start + 39}`);
}

function canListen(port) {
  return new Promise((resolve) => {
    const server = createServer()
      .once("error", () => resolve(false))
      .once("listening", () => {
        server.close(() => resolve(true));
      })
      .listen(port, host);
  });
}

function updateTauriDevUrl(devUrl) {
  const { configPath, config } = readTauriConfig();
  if (config.build.devUrl === devUrl) return;
  config.build.devUrl = devUrl;
  writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);
}

function restoreTauriDevUrl(devUrl) {
  try {
    updateTauriDevUrl(devUrl);
  } catch {
    // Do not mask the dev server exit code if config restoration fails.
  }
}

function readTauriConfig() {
  const configPath = new URL("../src-tauri/tauri.conf.json", import.meta.url);
  return {
    configPath,
    config: JSON.parse(readFileSync(configPath, "utf8")),
  };
}
