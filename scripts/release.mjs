import { parseArgs, styleText } from "node:util";
import { spawn } from "node:child_process";
import * as readline from "node:readline/promises";
import { stdin as input, stdout as output } from "node:process";

const { values: args } = parseArgs({
  args: process.argv.slice(2),
  options: {
    bump: {
      type: "string",
      default: "patch",
    },
    type: {
      type: "string",
    },
    packages: {
      short: "p",
      type: "string",
      multiple: true,
      default: [],
    },
    exclude: {
      short: "x",
      type: "string",
      multiple: true,
      default: [],
    },
  },
});

async function exec(cmd, args, opts = {}) {
  return new Promise((resolve, reject) => {
    let child = spawn(cmd, args, {
      shell: true,
      stdio: "inherit",
      ...opts,
    });
    let out = "";
    let err = "";

    child.stdout?.on("data", (data) => {
      out += data;
    });

    child.stderr?.on("data", (data) => {
      err += data;
    });

    child.on("close", (code) => {
      if (code == 0) {
        resolve({ err: err.trim(), out: out.trim() });
      } else {
        reject();
      }
    });
  });
}

async function runCargo(args, opts) {
  return (await exec("cargo", args, opts)).out;
}

async function getPackageNames() {
  let packages = [];

  if (args.packages.length > 0) {
    packages = args.packages;
  } else {
    let metadata = JSON.parse(
      await runCargo(["metadata", "--format-version", "1", "--no-deps", "--no-default-features"], {
        stdio: "pipe",
      }),
    );

    packages = metadata.packages.map((pkg) => pkg.name);
  }

  if (args.type) {
    packages = packages.filter((pkg) => pkg.endsWith(args.type));
  }

  // Common crates are not plugins
  return packages.filter((pkg) => !pkg.includes("common") && !args.exclude.includes(pkg));
}

let packages = await getPackageNames();

if (packages.length == 0) {
  throw new Error("No plugins to release!");
}

const rl = readline.createInterface({ input, output });
const answer = await rl.question(
  `Release (${styleText("yellow", args.bump)}) plugins ${packages.map((pkg) => styleText("cyan", pkg)).join(", ")}? [Y/N] `,
);

rl.close();

if (answer.toLowerCase() == "n") {
  process.exit(0);
}

// We must release 1-by-1 and tag them individually,
// otherwise the GitHub release workflow doesn't work
for (let pkgName of packages) {
  console.log(`Releasing ${styleText("cyan", pkgName)}`);

  await runCargo([
    "release",
    args.bump,
    "--no-publish",
    "--no-confirm",
    "--execute",
    "-p",
    pkgName,
  ]);

  console.log();

  await new Promise((resolve) => {
    setTimeout(resolve, 3000);
  });
}

console.log(`Released ${styleText("green", String(packages.length))} plugins!`);
