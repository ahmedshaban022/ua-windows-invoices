#!/usr/bin/env node
/**
 * Generate the updater signing keypair **on the owner's machine**.
 *
 * The private key never leaves this machine: it is written to a file outside
 * the repository, with a password you choose, and it is never printed, never
 * committed and never pasted into a chat.
 *
 *   node scripts/generate-updater-key.mjs
 *
 * Afterwards:
 *   1. Copy the printed public key into `src-tauri/tauri.conf.json`
 *      (`plugins.updater.pubkey`).
 *   2. Set the private key file content as the CI secret
 *      `TAURI_SIGNING_PRIVATE_KEY` and its password as
 *      `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
 */
import { spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const keyDir = join(homedir(), ".tauri");
const keyFile = join(keyDir, "invoice-updater.key");

function fail(message) {
  console.error(`\n✖ ${message}\n`);
  process.exit(1);
}

if (existsSync(keyFile)) {
  fail(
    `The private key already exists at:\n   ${keyFile}\n` +
      "Delete it first (or move it) if you really want to create a new one.",
  );
}

mkdirSync(keyDir, { recursive: true });

console.log("\nGenerating a new updater signing key…");
console.log("You will be asked for a password. Use a long, unique password and");
console.log("store it in your password manager — you will need it in CI too.\n");

const result = spawnSync(
  process.platform === "win32" ? "npx.cmd" : "npx",
  ["tauri", "signer", "generate", "--write-keys", keyFile],
  { stdio: ["inherit", "pipe", "inherit"], shell: process.platform === "win32" },
);

if (result.error) fail(`Could not run the Tauri CLI: ${result.error.message}`);
if (result.status !== 0) fail("The Tauri CLI could not generate the key.");

const output = result.stdout?.toString() ?? "";

// The CLI prints:
//   Your public key: <base64>
//   Your private key: <base64>
const publicKey = output.match(/public key:\s*(\S+)/i)?.[1];
const privateKey = output.match(/private key:\s*(\S+)/i)?.[1];

if (!publicKey) fail("Could not read the public key from the Tauri CLI output.");

if (privateKey) {
  // Keep a copy in the standard location the CLI understands, but never in
  // the repository.
  writeFileSync(`${keyFile}.pub`, `${publicKey}\n`, "utf8");
}

try {
  chmodSync(keyFile, 0o600);
} catch {
  // Windows ignores POSIX permissions; the file still lives outside the repo.
}

console.log("\n──────────────────────────────────────────────");
console.log(" Public key (safe to commit):");
console.log(`   ${publicKey}`);
console.log("──────────────────────────────────────────────");
console.log("\n Private key written to:");
console.log(`   ${keyFile}`);
console.log("\nNext steps:");
console.log(" 1. Set `plugins.updater.pubkey` in src-tauri/tauri.conf.json to the");
console.log("    public key above.");
console.log(" 2. Add the private key file content as the repository secret");
console.log("    TAURI_SIGNING_PRIVATE_KEY (GitHub: Settings → Secrets).");
console.log(" 3. Add the password as TAURI_SIGNING_PRIVATE_KEY_PASSWORD.");
console.log("\n Do NOT commit the private key and do NOT paste it into a chat.\n");

// A tiny sanity check that the key file exists and is not empty.
const written = readFileSync(keyFile, "utf8");
if (written.trim().length === 0) fail("The private key file is empty.");
