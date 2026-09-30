/// Artifact keys describe the executable ABI independently of service management.
pub const ARTIFACT_TARGETS: &[&str] = &[
    "amd64",
    "arm64",
    "linux-gnu-amd64",
    "linux-gnu-arm64",
    "linux-musl-amd64",
    "linux-musl-arm64",
    "macos-arm64",
    "freebsd-amd64",
    "freebsd-arm64",
    "windows-amd64",
    "windows-arm64",
];

pub fn artifact_target(os: &str, libc: Option<&str>, arch: &str) -> Option<String> {
    let arch = match arch {
        "x86_64" | "amd64" => "amd64",
        "aarch64" | "arm64" => "arm64",
        _ => return None,
    };
    let platform = match os {
        "linux" => match libc {
            Some("musl") => "linux-musl",
            Some("gnu" | "glibc") => "linux-gnu",
            _ => return None,
        },
        "macos" if arch == "arm64" => "macos",
        "freebsd" => "freebsd",
        "windows" => "windows",
        _ => return None,
    };
    Some(format!("{platform}-{arch}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_keys_require_a_supported_platform_and_abi() {
        assert_eq!(
            artifact_target("linux", Some("musl"), "x86_64").as_deref(),
            Some("linux-musl-amd64")
        );
        assert_eq!(
            artifact_target("linux", Some("glibc"), "aarch64").as_deref(),
            Some("linux-gnu-arm64")
        );
        assert_eq!(
            artifact_target("freebsd", None, "arm64").as_deref(),
            Some("freebsd-arm64")
        );
        assert!(artifact_target("linux", None, "amd64").is_none());
        assert!(artifact_target("macos", None, "amd64").is_none());
        assert!(artifact_target("windows", None, "riscv64").is_none());
    }
}
