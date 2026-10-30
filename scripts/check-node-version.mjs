#!/usr/bin/env node
// Compares the running Node version against package.json's engines.node floor.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pkg = JSON.parse(readFileSync(join(here, "..", "package.json"), "utf8"));
const required = pkg.engines?.node ?? "";

const match = required.match(/^>=(\d+)\.(\d+)\.(\d+)$/);
if (!match) {
  console.error(
    `check-node-version: package.json engines.node "${required}" is not a ">=X.Y.Z" range this script understands.`
  );
  process.exit(1);
}

const requiredParts = match.slice(1).map(Number);
const actualParts = process.versions.node.split(".").map(Number);

let meetsFloor = true;
for (let i = 0; i < 3; i++) {
  if (actualParts[i] > requiredParts[i]) break;
  if (actualParts[i] < requiredParts[i]) {
    meetsFloor = false;
    break;
  }
}

if (!meetsFloor) {
  console.error(
    `check-node-version: Node ${process.versions.node} is below the required ${required} (package.json engines.node).\n` +
      "Run: nvm install 26 --reinstall-packages-from=current"
  );
  process.exit(1);
}
