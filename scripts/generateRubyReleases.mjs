// @ts-check
import fs from "node:fs";

const GH_TOKEN = process.env.GITHUB_TOKEN || process.env.GH_TOKEN;
const RELEASES_PATH = "tools/ruby/releases.json";
const data = fs.existsSync(RELEASES_PATH)
  ? JSON.parse(fs.readFileSync(RELEASES_PATH, "utf8"))
  : {};

const headers = {
  Accept: "application/vnd.github+json",
  ...(GH_TOKEN ? { Authorization: `Bearer ${GH_TOKEN}` } : {}),
};

let page = 1;

while (true) {
  console.log(`Loading page ${page}`);

  const response = await fetch(
    `https://api.github.com/repos/jdx/ruby/releases?per_page=100&page=${page}`,
    { headers },
  );

  if (!response.ok) {
    throw new Error(`GitHub API returned HTTP ${response.status}`);
  }

  const releases = await response.json();

  for (const release of releases) {
    const version = release.tag_name;

    // Ignore immutable build revisions, such as 3.4.9-1.
    if (!/^\d+\.\d+\.\d+(?:-(?:preview|rc)\d+)?$/.test(version)) {
      continue;
    }

    for (const asset of release.assets) {
      const match = asset.name.match(
        /^ruby-(.+)\.(macos|x86_64_linux|arm64_linux)\.tar\.gz$/,
      );

      if (!match || match[1] !== version) {
        continue;
      }

      data[version] ||= {};
      data[version][match[2]] = asset.name;
    }
  }

  const link = response.headers.get("link");

  if (!link?.includes('rel="next"')) {
    break;
  }

  page += 1;
}

function sortObjectKeys(value) {
  return Object.fromEntries(
    Object.entries(value)
      .sort(([a], [b]) => a.localeCompare(b, undefined, { numeric: true }))
      .map(([key, item]) => [key, typeof item === "object" ? sortObjectKeys(item) : item]),
  );
}

fs.writeFileSync(RELEASES_PATH, `${JSON.stringify(sortObjectKeys(data), null, 2)}\n`);
