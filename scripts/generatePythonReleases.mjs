import fs from "fs";

const OPT_LEVELS = ["pgo+lto", "pgo", "lto", "noopt"];
const GH_TOKEN = process.env.GITHUB_TOKEN || process.env.GH_TOKEN;

const response = await fetch(
  "https://api.github.com/repos/astral-sh/python-build-standalone/releases?per_page=100",
  {
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: GH_TOKEN ? `Bearer ${GH_TOKEN}` : undefined,
    },
  },
);
const releases = await response.json();
const data = {};

function mapTriple(triple) {
  switch (triple) {
    case "aarch64-apple-darwin":
      return "aarch64-apple-darwin";

    case "aarch64-unknown-linux-gnu":
      return "aarch64-unknown-linux-gnu";

    case "i686-pc-windows-msvc":
    case "i686-pc-windows-msvc-shared":
    case "i686-pc-windows-msvc-static":
      return "i686-pc-windows-msvc";

    case "i686-unknown-linux-gnu":
      return "i686-unknown-linux-gnu";

    case "macos":
    case "x86_64-apple-darwin":
      return "x86_64-apple-darwin";

    case "windows-amd64":
    case "windows-amd64-shared":
    case "windows-amd64-static":
    case "windows-x86":
    case "windows-x86-shared":
    case "windows-x86-static":
    case "x86_64-pc-windows-msvc":
    case "x86_64-pc-windows-msvc-shared":
    case "x86_64-pc-windows-msvc-static":
      return "x86_64-pc-windows-msvc";

    case "linux64":
    case "x86_64-unknown-linux-gnu":
    case "x86_64_v2-unknown-linux-gnu":
    case "x86_64_v3-unknown-linux-gnu":
    case "x86_64_v4-unknown-linux-gnu":
      return "x86_64-unknown-linux-gnu";

    case "linux64-musl":
    case "x86_64-unknown-linux-musl":
    case "x86_64_v2-unknown-linux-musl":
    case "x86_64_v3-unknown-linux-musl":
    case "x86_64_v4-unknown-linux-musl":
      return "x86_64-unknown-linux-musl";

    case "ppc64le-unknown-linux-gnu":
      return "powerpc64le-unknown-linux-gnu";

    case "s390x-unknown-linux-gnu":
      return "s390x-unknown-linux-gnu";

    case "armv7-unknown-linux-gnueabi":
      return "armv7-unknown-linux-gnueabi";

    case "armv7-unknown-linux-gnueabihf":
      return "armv7-unknown-linux-gnueabihf";

    case "riscv64-unknown-linux-gnu":
      return "riscv64gc-unknown-linux-gnu";

    default:
      throw new Error(`Unknown triple ${triple}`);
  }
}

function mapVersion(version) {
  let parts = version.match(/(\d+)\.(\d+)(?:\.(\d+))(?:([a-z]+)([0-9]+))?/);
  let value = `${parts[1]}.${parts[2]}.${parts[3] || 0}`;

  if (parts[4]) {
    value += `-${parts[4]}.${parts[5]}`;
  }

  return value;
}

function extractTripleInfo(assetName, releaseName) {
  let name = assetName.replace("cpython-", "");
  let version = "";
  let triple = "";
  let sha256 = false;

  // Newer releases:
  //   cpython-3.10.2+20220227-aarch64-apple-darwin-debug-full.tar.zst.sha256
  if (name.includes(`+${releaseName}`)) {
    let parts = name.split(`+${releaseName}-`);
    version = parts[0];

    parts = parts[1].split("-");
    sha256 = parts.pop().endsWith(".sha256");
    triple = parts.filter((p) => !OPT_LEVELS.includes(p)).join("-");

    // Older releases:
    //   cpython-3.7.3-linux64-20190427T2308.tar.zst
    //   cpython-3.7.3-windows-amd64-20190430T0616.tar.zst
  } else {
    const parts = name.split("-");
    version = parts.shift();
    sha256 = parts.pop().endsWith(".sha256");
    triple = parts.filter((p) => !OPT_LEVELS.includes(p)).join("-");
  }

  return {
    triple: mapTriple(triple),
    version: mapVersion(version),
    sha256,
  };
}

function processAssets(assets, releaseName, optLevel) {
  assets.forEach((asset) => {
    const { version, triple, sha256 } = extractTripleInfo(asset.name, releaseName);

    if (!data[version]) {
      data[version] = {};
    }

    if (!data[version][triple]) {
      data[version][triple] = {
        download: null,
        checksum: null,
      };
    }

    if (sha256) {
      if (
        typeof data[version][triple].download === "string" &&
        data[version][triple].download.includes(optLevel) &&
        !data[version][triple].checksum
      ) {
        data[version][triple].checksum = asset.browser_download_url;
      }
    } else if (!data[version][triple].download) {
      data[version][triple].download = asset.browser_download_url;
    }
  });
}

const FILTER_WORDS = [
  "freethreaded",
  "debug",
  "install_only",
  "msvc-static",
  "_v2-",
  "_v3-",
  "_v4-",
];

releases.forEach((release) => {
  // Remove debug, install only, and unwanted builds
  const assets = release.assets.filter((asset) =>
    FILTER_WORDS.every((word) => !asset.name.includes(word)),
  );

  // Process assets in order of most wanted to least wanted
  OPT_LEVELS.forEach((optLevel) => {
    processAssets(
      assets.filter((asset) => asset.name.includes(optLevel)),
      release.name,
      optLevel,
    );
  });
});

fs.writeFileSync("tools/python/releases.json", JSON.stringify(data, null, 2));
