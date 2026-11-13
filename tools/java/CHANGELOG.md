# Changelog

## Unreleased

#### 💥 Breaking

- Requires proto v0.64 or later.
- Removed the `debian`, `graalvm-ce20`, and `zulu-prime` distributions, which foojay no longer
  lists or returns packages for.
- The default distribution is now `temurin` instead of `openjdk`, which applies to versions without a
  vendor, like `21`. jdk.java.net's `openjdk` builds stop receiving updates 6 months after a release,
  so `21` resolved to 21.0.2 from January 2024. Use `openjdk-21` to keep the previous behavior.

#### 🚀 Updates

- Updated to support proto v0.64 release.
- Versions with a 4th number, like `18.0.2.1`, now use proto's revision support, instead of
  encoding it into the pre-release (`18.0.2-v1`), including those in `.sdkmanrc`. Existing pins
  with the encoding still resolve, to the revision.

#### 🐞 Fixes

- Fixed Linux ppc64 requesting big-endian builds, which most vendors don't publish, instead of the
  little-endian (`ppc64le`) builds that Linux uses.
- Fixed comments in `.java-version`, like `21 # LTS`, failing to parse. Everything after `#` is
  now ignored.
- Fixed versions without build metadata, like `temurin-21.0.11` or `21.0.2` in `.java-version`,
  failing to resolve, as every listed version has one (`21.0.11+10`). They now match any build.
- Fixed `.sdkmanrc` parsing:
  - Supports SDKMAN's newer identifiers, like `21.0.13.0+9-tem`, which failed to parse.
  - Supports the `jbr` (JetBrains) vendor.
  - Ignores comments and whitespace, like SDKMAN does.
- Fixed legacy `1.x` versions in `.java-version`, like `1.8`, which never resolved.
- Fixed installs of SapMachine and Tencent Kona failing checksum verification, as proto can't detect
  the algorithm of SapMachine's checksum files, nor verify Kona's md5 checksums.
- Fixed versions that require a login to download, like Oracle JDK 17.0.13 and later, being listed
  and installed with an empty URL.
- Removed `$JAVA_HOME/bin` from the globals directories. It's another JDK, not where globals are
  installed, and was added to `PATH`.

#### ⚙️ Internal

- Version files are now read from their path, as the `content` input is deprecated.

## 0.1.1

#### 🚀 Updates

- Updated to support proto v0.60 release.

## 0.1.0

#### 🚀 Updates

- Added initial Java support.
