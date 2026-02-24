#!/usr/bin/env bun
"use strict";

import * as https from "https";
import { execSync, spawnSync } from "child_process";
import { existsSync, mkdirSync, chmodSync, unlinkSync, createWriteStream, copyFileSync, readFileSync, writeFileSync } from "fs";
import { join } from "path";
import { homedir, tmpdir } from "os";
import { IncomingMessage } from "http";

const pkg = require("../package.json") as { version: string };
const VERSION: string = pkg.version;
const REPO = "ProjectAI00/ai-db-imi";
const BIN_DIR = join(homedir(), ".local", "bin");
const BIN = join(BIN_DIR, "imi");

function getTarget(): string {
  const { platform, arch } = process;
  if (platform === "darwin" && arch === "arm64") return "aarch64-apple-darwin";
  if (platform === "darwin" && arch === "x64") return "x86_64-apple-darwin";
  if (platform === "linux" && arch === "x64") return "x86_64-unknown-linux-musl";
  if (platform === "linux" && arch === "arm64") return "aarch64-unknown-linux-musl";
  console.error(`Unsupported platform: ${platform} ${arch}`);
  process.exit(1);
}

function fetch(url: string, dest: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const file = createWriteStream(dest);
    const req = (u: string) =>
      https.get(u, (res: IncomingMessage) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          return req(res.headers.location as string);
        }
        if (res.statusCode !== 200) {
          reject(new Error(`HTTP ${res.statusCode} for ${u}`));
          return;
        }
        res.pipe(file);
        file.on("finish", () => file.close(resolve as () => void));
      }).on("error", reject);
    req(url);
  });
}

async function main(): Promise<void> {
  const target = getTarget();
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/imi-${target}.tar.gz`;
  const tmp = join(tmpdir(), `imi-${Date.now()}.tar.gz`);

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

  const inPath = (process.env.PATH || "").split(":").includes(BIN_DIR);
  if (!inPath) {
    console.log(`\nAdd to your shell config:\n  export PATH="$HOME/.local/bin:$PATH"\n`);
  }

  runInit();
}

function installSkills(): void {
  // The bundled SKILL.md lives next to this script inside the npm package
  const skillSrc = join(import.meta.dir, "skills", "imi", "SKILL.md");
  if (!existsSync(skillSrc)) return;

  // Agent CLIs that follow the open agentskills standard
  const targets: { name: string; dir: string }[] = [
    { name: "GitHub Copilot CLI", dir: join(homedir(), ".copilot", "skills", "imi") },
    { name: "Claude Code",        dir: join(homedir(), ".claude",  "skills", "imi") },
  ];

  const installed: string[] = [];
  const skipped: string[] = [];

  for (const { name, dir } of targets) {
    const parentDir = join(dir, "..");          // ~/.copilot/skills or ~/.claude/skills
    const parentExists = existsSync(join(parentDir, ".."));  // ~/.copilot or ~/.claude

    if (!parentExists) {
      skipped.push(name);
      continue;
    }

    mkdirSync(dir, { recursive: true });
    copyFileSync(skillSrc, join(dir, "SKILL.md"));
    installed.push(name);
  }

  if (installed.length > 0) {
    console.log(`\nAgent skills installed into: ${installed.join(", ")}`);
    console.log(`Agents will now automatically run imi commands when you mention "imi".`);
  }
  if (skipped.length > 0) {
    console.log(`Skipped (not installed): ${skipped.join(", ")}`);
  }

  // Plugin registration
  console.log(`\nPlugin setup:`);
  registerClaudePlugin();
  if (existsSync(join(homedir(), ".copilot"))) {
    console.log(`  GitHub Copilot CLI: run /plugin marketplace add ProjectAI00/ai-db-imi then /plugin install imi`);
  }
}

function registerClaudePlugin(): void {
  const claudePluginsDir = join(homedir(), ".claude", "plugins");
  const knownFile = join(claudePluginsDir, "known_marketplaces.json");

  if (!existsSync(claudePluginsDir)) return;

  // Read existing marketplaces
  let known: Record<string, unknown> = {};
  try { known = JSON.parse(readFileSync(knownFile, "utf8")); } catch {}

  if (known["imi"]) return; // already registered

  const installLocation = join(claudePluginsDir, "marketplaces", "imi");

  // Clone the repo so Claude Code can read the plugin manifest
  if (!existsSync(installLocation)) {
    try {
      execSync(
        `git clone --depth 1 https://github.com/ProjectAI00/ai-db-imi "${installLocation}"`,
        { stdio: "pipe" }
      );
    } catch {
      // git unavailable or no network — register the source URL anyway
    }
  }

  known["imi"] = {
    source: { source: "github", repo: "ProjectAI00/ai-db-imi" },
    installLocation,
    lastUpdated: new Date().toISOString(),
  };
  writeFileSync(knownFile, JSON.stringify(known, null, 2));
  console.log(`  Claude Code: marketplace registered → run /plugin install imi to activate`);
}

function runInit(): void {
  installSkills();
  const result = spawnSync(BIN, ["init"], { stdio: "inherit" });
  process.exit(result.status ?? 0);
}

main().catch((err: Error) => {
  console.error("\nInstall failed:", err.message);
  console.error(`Manual install: curl -fsSL https://aibyimi.com/install | bash`);
  process.exit(1);
});
