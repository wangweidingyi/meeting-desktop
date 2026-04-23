import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { spawn } from "node:child_process";

function loadEnvFile(envPath) {
  if (!existsSync(envPath)) {
    return;
  }

  const contents = readFileSync(envPath, "utf8");
  for (const rawLine of contents.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) {
      continue;
    }

    const separator = line.indexOf("=");
    if (separator === -1) {
      continue;
    }

    const key = line.slice(0, separator).trim();
    let value = line.slice(separator + 1).trim();

    if (
      (value.startsWith("\"") && value.endsWith("\"")) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }

    if (!process.env[key]) {
      process.env[key] = value;
    }
  }
}

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const desktopRoot = path.resolve(scriptDir, "..");
const meetingRoot = path.resolve(scriptDir, "..", "..");
const serverRoot = path.join(meetingRoot, "meeting-server");
loadEnvFile(path.join(desktopRoot, ".env"));
loadEnvFile(path.join(serverRoot, ".env"));

if (!process.env.MEETING_SERVER_UDP_HOST && process.env.MEETING_UDP_HOST) {
  process.env.MEETING_SERVER_UDP_HOST = process.env.MEETING_UDP_HOST;
}

if (!process.env.MEETING_SERVER_UDP_PORT && process.env.MEETING_UDP_PORT) {
  process.env.MEETING_SERVER_UDP_PORT = process.env.MEETING_UDP_PORT;
}

if (!process.env.MEETING_SERVER_MQTT_BROKER) {
  const embedded = (process.env.MEETING_MQTT_EMBEDDED ?? "").trim().toLowerCase();
  const listenPort = process.env.MEETING_MQTT_LISTEN_PORT ?? "1883";
  if (embedded === "true" || embedded === "1" || embedded === "yes") {
    process.env.MEETING_SERVER_MQTT_BROKER = `tcp://127.0.0.1:${listenPort}`;
  }
}

const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
const child = spawn(npmCommand, ["run", "tauri", "dev"], {
  cwd: desktopRoot,
  env: process.env,
  stdio: "inherit",
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 1);
});
