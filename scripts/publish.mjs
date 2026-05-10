// Build and publish WASM plugins to GitHub Container Registry

// @ts-check
import { styleText } from "node:util";
import { join } from "node:path";
import { existsSync } from "node:fs";
import * as fs from "node:fs/promises";
import * as readline from "node:readline/promises";
import { stdin as input, stdout as output } from "node:process";
import { getArgs, getPackages, exec, execMoon } from "./utils.mjs";

const args = getArgs();
const packages = await getPackages(args);

if (packages.length == 0) {
  throw new Error("No plugins to publish!");
}

const rl = readline.createInterface({ input, output });

const answer = await rl.question(
  `Publish (${styleText("yellow", args.bump)}) plugins ${packages
    .map((pkg) => styleText("cyan", pkg.name))
    .join(", ")}? [Y/N] `,
);

rl.close();

if (answer.toLowerCase() == "n") {
  process.exit(0);
}

const TARGET_DIR = process.env.CARGO_TARGET_DIR || join(process.cwd(), "target");

for (let pkg of packages) {
  console.log(`Publishing ${styleText("cyan", pkg.name)}`);

  await execMoon(["run", `${pkg.name}:build`]);

  // Build OCI annotations
  // https://docs.github.com/en/packages/learn-github-packages/connecting-a-repository-to-a-package
  const annosPath = join(pkg.root, "annotations.json");
  const annos = {
    "moonrepo.runtime":
      pkg.name.endsWith("toolchain") || pkg.name.endsWith("extension") ? "moon" : "proto",
    "moonrepo.plugin.type": pkg.name.split("_")[1],
    "moonrepo.plugin.format": "wasm",
    "org.opencontainers.image.vendor": "moonrepo",
    "org.opencontainers.image.version": pkg.version,
    "org.opencontainers.image.title": pkg.name,
    "org.opencontainers.image.description": pkg.description || undefined,
    "org.opencontainers.image.licenses": pkg.license || undefined,
    "org.opencontainers.image.source": pkg.repository || undefined,
    "org.opencontainers.image.documentation": pkg.documentation || undefined,
    "org.opencontainers.image.url": pkg.homepage || pkg.repository || undefined,
    "org.opencontainers.image.authors":
      pkg.authors && pkg.authors.length > 0 ? pkg.authors.join(", ") : undefined,
  };

  await fs.writeFile(
    annosPath,
    JSON.stringify({
      $manifest: annos,
    }),
  );

  // Push to registry with multiple layers
  const wasmFile = join(TARGET_DIR, `wasm32-wasip1/release/${pkg.name}.wasm`);
  const readmeFile = join(pkg.root, "README.md");

  const args = [
    "push",
    "--debug",
    "--disable-path-validation",
    "--annotation-file",
    annosPath,
    "--artifact-type",
    "application/wasm",
    `ghcr.io/moonrepo/${pkg.name}:${pkg.version},latest`,
    `${wasmFile}:application/wasm`,
  ];

  if (existsSync(readmeFile)) {
    args.push(`${readmeFile}:text/markdown`);
  }

  await exec("oras", args);

  console.log();

  await new Promise((resolve) => {
    setTimeout(resolve, 3000);
  });
}

console.log(`Published ${styleText("green", String(packages.length))} plugins!`);
