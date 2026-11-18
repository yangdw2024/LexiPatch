#!/usr/bin/env node
"use strict";

const { spawnSync } = require("child_process");

function findPython() {
  const candidates = process.platform === "win32" ? ["python", "py"] : ["python3", "python"];
  for (const cmd of candidates) {
    const res = spawnSync(cmd, ["--version"], { stdio: "ignore" });
    if (res.status === 0) return cmd;
  }
  return null;
}

function main() {
  const py = findPython();
  if (!py) {
    console.error("[lexipatch] Python 3 is required. Please install Python 3.9+.");
    process.exit(1);
  }

  const check = spawnSync(py, ["-c", "import lexipatch"], { stdio: "ignore" });
  if (check.status !== 0) {
    console.error("[lexipatch] Installing lexipatch Python engine from PyPI...");
    const install = spawnSync(py, ["-m", "pip", "install", "--quiet", "lexipatch"], {
      stdio: "inherit",
    });
    if (install.status !== 0) {
      console.error("[lexipatch] Failed to install lexipatch via pip.");
      process.exit(install.status || 1);
    }
  }

  const args = ["-m", "lexipatch.cli", ...process.argv.slice(2)];
  const run = spawnSync(py, args, { stdio: "inherit" });
  process.exit(run.status === null ? 1 : run.status);
}

main();
