// @ts-check
import { parseArgs } from "node:util";
import { dirname } from "node:path";
import { spawn } from "node:child_process";

export function getArgs() {
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

  return args;
}

/**
 * @param {string} cmd
 * @param {string[]} args
 * @param {import("node:child_process").SpawnOptions} [options]
 * @returns {Promise<{out: string, err: string}>}
 */
export async function exec(cmd, args, options = {}) {
  return new Promise((resolve, reject) => {
    let child = spawn(cmd, args, {
      shell: true,
      stdio: "inherit",
      ...options,
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

/**
 * @param {string[]} args
 * @param {import("node:child_process").SpawnOptions} [opts]
 * @returns {Promise<string>}
 */
export async function execCargo(args, opts) {
  return (await exec("cargo", args, opts)).out;
}

/**
 * @param {{packages?: string[], exclude?: string[], type?: string}} args
 * @returns {Promise<{name: string, version: string, root: string}[]>}
 */
export async function getPackages(args) {
  let packages = [];
  let metadata = JSON.parse(
    await execCargo(
      [
        "metadata",
        "--format-version",
        "1",
        "--no-deps",
        "--no-default-features",
      ],
      {
        stdio: "pipe",
      }
    )
  );

  for (let pkg of metadata.packages) {
    if (
      args.packages &&
      args.packages.length > 0 &&
      !args.packages.includes(pkg.name)
    ) {
      continue;
    }

    packages.push({
      name: pkg.name,
      version: pkg.version,
      root: dirname(pkg.manifest_path),
    });
  }

  if (args.type) {
    packages = packages.filter((pkg) => pkg.name.endsWith(args.type));
  }

  // Common crates are not plugins
  return packages.filter(
    (pkg) => !pkg.name.includes("common") && !args.exclude?.includes(pkg.name)
  );
}
