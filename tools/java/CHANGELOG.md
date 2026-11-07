# Changelog

## Unreleased

#### 💥 Breaking

- The default distribution is now `temurin` instead of `openjdk`, which applies to versions without a
  vendor, like `21`. jdk.java.net's `openjdk` builds stop receiving updates 6 months after a release,
  so `21` resolved to 21.0.2 from January 2024. Use `openjdk-21` to keep the previous behavior.

#### 🐞 Fixes

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

## 0.1.1

#### 🚀 Updates

- Updated to support proto v0.60 release.

## 0.1.0

#### 🚀 Updates

- Added initial Java support.
