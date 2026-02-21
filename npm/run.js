#!/usr/bin/env node
"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
const https = __importStar(require("https"));
const child_process_1 = require("child_process");
const fs_1 = require("fs");
const path_1 = require("path");
const os_1 = require("os");
const pkg = require("../package.json");
const VERSION = pkg.version;
const REPO = "ProjectAI00/ai-db-imi";
const BIN_DIR = (0, path_1.join)((0, os_1.homedir)(), ".local", "bin");
const BIN = (0, path_1.join)(BIN_DIR, "imi");
function getTarget() {
    const { platform, arch } = process;
    if (platform === "darwin" && arch === "arm64")
        return "aarch64-apple-darwin";
    if (platform === "darwin" && arch === "x64")
        return "x86_64-apple-darwin";
    if (platform === "linux" && arch === "x64")
        return "x86_64-unknown-linux-musl";
    if (platform === "linux" && arch === "arm64")
        return "aarch64-unknown-linux-musl";
    console.error(`Unsupported platform: ${platform} ${arch}`);
    process.exit(1);
}
function fetch(url, dest) {
    return new Promise((resolve, reject) => {
        const file = (0, fs_1.createWriteStream)(dest);
        const req = (u) => https.get(u, (res) => {
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
    const tmp = (0, path_1.join)((0, os_1.tmpdir)(), `imi-${Date.now()}.tar.gz`);
    if ((0, fs_1.existsSync)(BIN)) {
        try {
            const installed = (0, child_process_1.execSync)(`${BIN} --version 2>/dev/null`, { encoding: "utf8" }).trim();
            if (installed.includes(VERSION)) {
                console.log(`imi ${VERSION} already installed`);
                runInit();
                return;
            }
        }
        catch (_a) { }
    }
    process.stdout.write(`Installing imi v${VERSION} for ${target}... `);
    await fetch(url, tmp);
    (0, fs_1.mkdirSync)(BIN_DIR, { recursive: true });
    (0, child_process_1.execSync)(`tar -xzf "${tmp}" -C "${BIN_DIR}"`, { stdio: "pipe" });
    (0, fs_1.chmodSync)(BIN, 0o755);
    (0, fs_1.unlinkSync)(tmp);
    console.log("done");
    const inPath = (process.env.PATH || "").split(":").includes(BIN_DIR);
    if (!inPath) {
        console.log(`\nAdd to your shell config:\n  export PATH="$HOME/.local/bin:$PATH"\n`);
    }
    runInit();
}
function runInit() {
    var _a;
    const result = (0, child_process_1.spawnSync)(BIN, ["init"], { stdio: "inherit" });
    process.exit((_a = result.status) !== null && _a !== void 0 ? _a : 0);
}
main().catch((err) => {
    console.error("\nInstall failed:", err.message);
    console.error(`Manual install: curl -fsSL https://aibyimi.com/install | bash`);
    process.exit(1);
});
