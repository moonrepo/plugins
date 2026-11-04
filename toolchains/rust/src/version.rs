use moon_pdk_api::UnresolvedVersionSpec;
use proto_pdk_api::{Op, VersionKind};

/// Convert to a Cargo `rust-version` (MSRV), which only accepts
/// a bare `X.Y` or `X.Y.Z`. Channels and ranges return `None`.
pub fn to_msrv(spec: &UnresolvedVersionSpec) -> Option<String> {
    match spec {
        UnresolvedVersionSpec::Version(version)
            if version.kind == VersionKind::Semantic && version.scope.is_none() =>
        {
            Some(format!(
                "{}.{}.{}",
                version.major, version.minor, version.patch
            ))
        }
        UnresolvedVersionSpec::Requirement(req)
            if req.kind == VersionKind::Semantic
                && req.scope.is_none()
                && matches!(req.op, Op::Tilde | Op::Caret | Op::Exact | Op::GreaterEq) =>
        {
            let (major, minor) = (req.major?, req.minor?);

            Some(match req.patch {
                Some(patch) => format!("{major}.{minor}.{patch}"),
                None => format!("{major}.{minor}"),
            })
        }
        _ => None,
    }
}

/// Convert to a rustup toolchain channel: `stable`, `beta`, `nightly`,
/// a dated channel, or a fully-qualified version. Partial versions like
/// `1.80` return `None`, as rustup treats them as a separate toolchain.
pub fn to_channel(spec: &UnresolvedVersionSpec) -> Option<String> {
    if spec.is_canary() {
        return Some("nightly".into());
    }

    match spec {
        UnresolvedVersionSpec::Alias(alias)
            if matches!(alias.as_str(), "stable" | "beta" | "nightly") =>
        {
            Some(alias.to_string())
        }
        // `nightly-2024-01-01` is a calendar version with a scope
        UnresolvedVersionSpec::Version(version)
            if (version.kind == VersionKind::Semantic && version.scope.is_none())
                || (version.kind == VersionKind::Calendar && version.scope.is_some()) =>
        {
            Some(spec.to_string())
        }
        _ => None,
    }
}
