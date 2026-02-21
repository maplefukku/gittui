#!/usr/bin/env node

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");
const { createGunzip } = require("zlib");
const { pipeline } = require("stream");
const { promisify } = require("util");
const pipelineAsync = promisify(pipeline);

const VERSION = require("./package.json").version;
const REPO = "maplefukku/gittui";

const PLATFORM_MAP = {
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "linux-x64": "x86_64-unknown-linux-gnu",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "win32-x64": "x86_64-pc-windows-msvc",
};

function getTarget() {
  const key = `${process.platform}-${process.arch}`;
  const target = PLATFORM_MAP[key];
  if (!target) {
    console.error(`Unsupported platform: ${key}`);
    console.error(`Supported platforms: ${Object.keys(PLATFORM_MAP).join(", ")}`);
    process.exit(1);
  }
  return target;
}

function getDownloadUrl(target) {
  const ext = process.platform === "win32" ? "zip" : "tar.gz";
  return `https://github.com/${REPO}/releases/download/v${VERSION}/gittui-${target}.${ext}`;
}

function httpsGet(url) {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: { "User-Agent": "gittui-npm" } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        httpsGet(res.headers.location).then(resolve, reject);
        return;
      }
      if (res.statusCode !== 200) {
        reject(new Error(`HTTP ${res.statusCode}: ${url}`));
        return;
      }
      resolve(res);
    }).on("error", reject);
  });
}

async function extractTarGz(stream, destDir) {
  const tar = require("child_process");
  const tmpFile = path.join(destDir, "gittui.tar.gz");

  await pipelineAsync(stream, fs.createWriteStream(tmpFile));
  execSync(`tar xzf "${tmpFile}" -C "${destDir}"`);
  fs.unlinkSync(tmpFile);
}

async function extractZip(stream, destDir) {
  const tmpFile = path.join(destDir, "gittui.zip");

  await pipelineAsync(stream, fs.createWriteStream(tmpFile));

  // Use PowerShell on Windows
  execSync(
    `powershell -Command "Expand-Archive -Path '${tmpFile}' -DestinationPath '${destDir}' -Force"`,
  );
  fs.unlinkSync(tmpFile);
}

async function main() {
  const target = getTarget();
  const url = getDownloadUrl(target);
  const binDir = path.join(__dirname, "bin");

  console.log(`Downloading gittui v${VERSION} for ${target}...`);

  fs.mkdirSync(binDir, { recursive: true });

  const stream = await httpsGet(url);

  if (process.platform === "win32") {
    await extractZip(stream, binDir);
  } else {
    await extractTarGz(stream, binDir);
  }

  // Make binary executable on Unix
  if (process.platform !== "win32") {
    const binPath = path.join(binDir, "gittui");
    fs.chmodSync(binPath, 0o755);
  }

  console.log("gittui installed successfully!");
}

main().catch((err) => {
  console.error("Failed to install gittui:", err.message);
  console.error("");
  console.error("You can install manually:");
  console.error("  cargo install gittui");
  console.error("  # or download from https://github.com/maplefukku/gittui/releases");
  process.exit(1);
});
