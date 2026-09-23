// `npm run tauri:build`, with updater signing skipped when there is no key.
//
// Tauri builds every bundle correctly and *then* fails on the signature if
// TAURI_SIGNING_PRIVATE_KEY is unset. The installers are sitting on disk, but
// the command exits non-zero and reads as a failed build — which is how a
// perfectly good local build gets thrown away and rerun.
//
// The key lives in GitHub Actions secrets, so CI signs and a development
// machine does not. Rather than asking everyone to remember a different
// command, this picks: no key, no updater artifacts, and it says which one it
// chose. Nothing about the app binary differs — updater artifacts are the
// separate .sig and archive files the updater downloads, not part of the MSI.
//
// Node only: this is build tooling and never ships (ADR 0001).

import { spawn } from "node:child_process";

const hasSigningKey = Boolean(process.env.TAURI_SIGNING_PRIVATE_KEY?.trim());
const passthrough = process.argv.slice(2);

const args = ["tauri", "build", ...passthrough];
if (!hasSigningKey) {
  args.push("--config", "src-tauri/tauri.noupdater.conf.json");
}

if (hasSigningKey) {
  console.log("Signing key present: building with updater artifacts.");
} else if (process.env.CI) {
  // Worth saying loudly in CI, where the usual cause is a fork without access
  // to the secret, but the other cause is a release job that has lost it.
  console.warn(
    "WARNING: TAURI_SIGNING_PRIVATE_KEY is not set in CI. " +
      "Installers will build, but this release cannot be auto-updated.",
  );
} else {
  console.log("No signing key: skipping updater artifacts. Installers are unaffected.");
}

const child = spawn("npx", args, { stdio: "inherit", shell: true });
child.on("exit", (code) => process.exit(code ?? 1));
