// @ts-check
import fs from "node:fs";

const OPT_LEVELS = ["install_only", "pgo+lto", "pgo", "lto", "lto+static", "noopt", "noopt+static"];
const GH_TOKEN = process.env.GITHUB_TOKEN || process.env.GH_TOKEN;

// Load the existing dataset so we can reduce the amount of API calls required
const data = {}; // JSON.parse(fs.readFileSync("tools/python/releases-v2.json", "utf8"));

// Fetch all the releases
const releases = [];
let page = 1;

while (true) {
  console.log(`Loading page ${page}`);

  const response = await fetch(
    `https://api.github.com/repos/astral-sh/python-build-standalone/releases?per_page=10&page=${page}`,
    {
      // @ts-expect-error
      headers: {
        Accept: "application/vnd.github+json",
        Authorization: GH_TOKEN ? `Bearer ${GH_TOKEN}` : undefined,
      },
    },
  );

  const link = response.headers.get("link");
  const text = await response.text();

  // The endpoint sometimes returns HTML, so we can't always parse it as JSON!
  if (text.startsWith("[")) {
    releases.push(...JSON.parse(text));
  } else {
    console.log(text);

    throw new Error("GitHub API returned a non-JSON response!");
  }

  if (link && !link.includes('rel="next"')) {
    break;
  }

  page += 1;
}

releases.sort((a, d) => a.id - d.id);

function mapTriple(triple) {
  switch (triple) {
    case "aarch64-apple-darwin":
      return "aarch64-apple-darwin";

    case "aarch64-unknown-linux-gnu":
      return "aarch64-unknown-linux-gnu";

    case "aarch64-unknown-linux-musl":
      return "aarch64-unknown-linux-musl";

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

    case "aarch64-pc-windows-msvc":
      return "aarch64-pc-windows-msvc";

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

function processAssets(assets, releaseName) {
  const releaseId = parseInt(releaseName);

  assets.sort();

  // Use the assets with the most wanted opt level first
  OPT_LEVELS.some((optLevel) => {
    const optAssets = assets.filter((asset) => asset.name.includes(optLevel));

    for (const asset of optAssets) {
      const { version, triple } = extractTripleInfo(asset.name, releaseName);

      if (!data[version]) {
        data[version] = {};
      }

      if (!data[version][triple]) {
        data[version][triple] = {};
      }

      const item = data[version][triple];

      item.file = `${releaseName}/${asset.name}`;

      // Uses SHA256SUMS in newer releases
      item.sha = releaseId >= 20220227 && releaseId < 20250708 ? true : undefined;
    }

    return optAssets.length > 0;
  });
}

const FILTER_WORDS = [
  "freethreaded",
  "debug",
  "install_only_stripped",
  "msvc-static",
  "_v2-",
  "_v3-",
  "_v4-",
  ".sha256",
];

releases.forEach((release) => {
  const releaseName = release.tag_name || release.name;

  processAssets(
    // Remove debug and unwanted builds
    release.assets.filter((asset) => FILTER_WORDS.every((word) => !asset.name.includes(word))),
    releaseName,
  );
});

fs.writeFileSync("tools/python/releases-v2.json", JSON.stringify(data, null, 2));
