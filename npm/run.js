#!/usr/bin/env node
"use strict";

const https = require("https");
const { execSync, spawnSync } = require("child_process");
const { existsSync, mkdirSync, chmodSync, unlinkSync, createWriteStream } = require("fs");
const { join } = require("path");
const { homedir, tmpdir } = require("os");

const VERSION = require("./package.json").version;
const REPO = "ProjectAI00/ai-db-imi";
const BIN_DIR = join(homedir(), ".local", "bin");
const BIN = join(BIN_DIR, "imi");

function getTarget() {
  const { platform, arch } = process;
  if (platform === "darwin" && arch === "arm64") return "aarch64-apple-darwin";
  if (platform === "darwin" && arch === "x64") return "x86_64-apple-darwin";
  if (platform === "linux" && arch === "x64") return "x86_64-unknown-linux-musl";
  if (platform === "linux" && arch === "arm64") return "aarch64-unknown-linux-musl";
  console.error(`Unsupported platform: ${platform} ${arch}`);
  process.exit(1);
}

function fetch(url, dest) {
  return new Promise((resolve, reject) => {
    const file = createWriteStream(dest);
    const req = (u) =>
      https.get(u, (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          return req(res.headers.location);
        }
        if (res.statusCode !== 200) {
          reject(new Error(`HTTP ${res.statusCode} for ${u}`));
          return;
        }
        res.pipe(file);
        file.on("finish", () => file.close(resolve));
      }).on("error", reject);
    req(url);
  });
}

async function main() {
  const target = getTarget();
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/imi-${target}.tar.gz`;
  const tmp = join(tmpdir(), `imi-${Date.now()}.tar.gz`);

  // Skip download if already the right version
  if (existsSync(BIN)) {
    try {
      const installed = execSync(`${BIN} --version 2>/dev/null`, { encoding: "utf8" }).trim();
      if (installed.includes(VERSION)) {
        console.log(`imi ${VERSION} already installed`);
        runInit();
        return;
      }
    } catch {}
  }

  process.stdout.write(`Installing imi v${VERSION} for ${target}... `);
  await fetch(url, tmp);
  mkdirSync(BIN_DIR, { recursive: true });
  execSync(`tar -xzf "${tmp}" -C "${BIN_DIR}"`, { stdio: "pipe" });
  chmodSync(BIN, 0o755);
  unlinkSync(tmp);
  console.log("done");

  // Ensure ~/.local/bin is on PATH hint
  const inPath = (process.env.PATH || "").split(":").includes(BIN_DIR);
  if (!inPath) {
    console.log(`\nAdd to your shell config:\n  export PATH="$HOME/.local/bin:$PATH"\n`);
  }

  runInit();
}

function runInit() {
  const result = spawnSync(BIN, ["init"], { stdio: "inherit" });
  process.exit(result.status ?? 0);
}

main().catch((err) => {
  console.error("\nInstall failed:", err.message);
  console.error(`Manual install: curl -fsSL https://aibyimi.com/install | bash`);
  process.exit(1);
});
