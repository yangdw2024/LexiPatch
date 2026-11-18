"use strict";
const { spawnSync } = require("child_process");

function runLexipatch(args) {
  const py = process.platform === "win32" ? "python" : "python3";
  const res = spawnSync(py, ["-m", "lexipatch.cli", ...args], { encoding: "utf-8" });
  if (res.status !== 0) {
    throw new Error(res.stderr || `lexipatch exited with ${res.status}`);
  }
  return res.stdout;
}

module.exports = { runLexipatch };
