// @ts-check
import { styleText } from "node:util";
import * as readline from "node:readline/promises";
import { stdin as input, stdout as output } from "node:process";
import { getArgs, getPackages, execCargo } from "./utils.mjs";

const args = getArgs();
const packages = await getPackages(args);

if (packages.length == 0) {
  throw new Error("No plugins to release!");
}

const rl = readline.createInterface({ input, output });

const answer = await rl.question(
  `Release (${styleText("yellow", args.bump)}) plugins ${packages
    .map((pkg) => styleText("cyan", pkg.name))
    .join(", ")}? [Y/N] `
);

rl.close();

if (answer.toLowerCase() == "n") {
  process.exit(0);
}

// We must release 1-by-1 and tag them individually,
// otherwise the GitHub release workflow doesn't work
for (let pkg of packages) {
  console.log(`Releasing ${styleText("cyan", pkg.name)}`);

  await execCargo([
    "release",
    args.bump,
    "--no-publish",
    "--no-confirm",
    "--execute",
    "-p",
    pkg.name,
  ]);

  console.log();

  await new Promise((resolve) => {
    setTimeout(resolve, 3000);
  });
}

console.log(`Released ${styleText("green", String(packages.length))} plugins!`);
