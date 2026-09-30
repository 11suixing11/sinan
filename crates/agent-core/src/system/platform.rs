use sinan_protocol::release::ReleaseError;

pub(crate) fn runtime_libc() -> Option<&'static str> {
    #[cfg(target_os = "linux")]
    {
        use std::{fs::File, io::Read, sync::OnceLock};
        static LIBC: OnceLock<&'static str> = OnceLock::new();
        Some(LIBC.get_or_init(|| {
            // Inspect the system userspace, not this possibly static Agent. No
            // external command or optional distribution utility is required.
            ["/bin/sh", "/usr/bin/env", "/bin/env"]
                .into_iter()
                .find_map(|path| {
                    let file = File::open(path).ok()?;
                    if !file.metadata().ok()?.is_file() {
                        return None;
                    }
                    let mut bytes = Vec::new();
                    file.take(64 * 1024).read_to_end(&mut bytes).ok()?;
                    elf_libc(&bytes)
                })
                .unwrap_or(if cfg!(target_env = "musl") {
                    "musl"
                } else {
                    "gnu"
                })
        }))
    }
    #[cfg(not(target_os = "linux"))]
    None
}

pub(crate) fn runtime_target() -> Result<String, ReleaseError> {
    sinan_protocol::platform::artifact_target(
        std::env::consts::OS,
        runtime_libc(),
        std::env::consts::ARCH,
    )
    .ok_or(ReleaseError::IdentityMismatch)
}

#[cfg(any(target_os = "linux", test))]
fn elf_libc(bytes: &[u8]) -> Option<&'static str> {
    // Both supported Linux architectures use ELF64 little endian. Read PT_INTERP
    // through its program header so arbitrary embedded strings cannot select an ABI.
    if bytes.get(..7)? != b"\x7fELF\x02\x01\x01" {
        return None;
    }
    let number = |offset: usize, length: usize| -> Option<usize> {
        let mut encoded = [0_u8; 8];
        encoded[..length].copy_from_slice(bytes.get(offset..offset.checked_add(length)?)?);
        usize::try_from(u64::from_le_bytes(encoded)).ok()
    };
    let start = number(32, 8)?;
    let stride = number(54, 2)?;
    let count = number(56, 2)?;
    if start < 64 || stride < 56 || count > 128 {
        return None;
    }
    for index in 0..count {
        let header = start.checked_add(index.checked_mul(stride)?)?;
        if number(header, 4)? != 3 {
            continue;
        }
        let offset = number(header.checked_add(8)?, 8)?;
        let length = number(header.checked_add(32)?, 8)?;
        if !(2..=4096).contains(&length) {
            return None;
        }
        let path = std::str::from_utf8(bytes.get(offset..offset.checked_add(length)?)?).ok()?;
        let path = path.strip_suffix('\0')?;
        if !path.starts_with('/') || path.contains('\0') {
            return None;
        }
        return match path.rsplit('/').next()? {
            "ld-linux-x86-64.so.2" | "ld-linux-aarch64.so.1" => Some("gnu"),
            "ld-musl-x86_64.so.1" | "ld-musl-aarch64.so.1" => Some("musl"),
            _ => None,
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::elf_libc;

    fn executable(interpreter: &str) -> Vec<u8> {
        let mut bytes = vec![0; 128];
        bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
        bytes[54..56].copy_from_slice(&56_u16.to_le_bytes());
        bytes[56..58].copy_from_slice(&1_u16.to_le_bytes());
        bytes[64..68].copy_from_slice(&3_u32.to_le_bytes());
        bytes[72..80].copy_from_slice(&128_u64.to_le_bytes());
        bytes[96..104].copy_from_slice(&(interpreter.len() as u64 + 1).to_le_bytes());
        bytes.extend_from_slice(interpreter.as_bytes());
        bytes.push(0);
        bytes
    }

    #[test]
    fn host_userspace_selects_libc_independently_of_agent_compilation() {
        for path in [
            "/lib64/ld-linux-x86-64.so.2",
            "/lib/ld-linux-aarch64.so.1",
            "/nix/store/example-glibc/lib/ld-linux-x86-64.so.2",
        ] {
            assert_eq!(elf_libc(&executable(path)), Some("gnu"));
        }
        for path in ["/lib/ld-musl-x86_64.so.1", "/lib/ld-musl-aarch64.so.1"] {
            assert_eq!(elf_libc(&executable(path)), Some("musl"));
        }
    }

    #[test]
    fn static_truncated_and_invalid_elf_cannot_claim_a_libc() {
        let bytes = executable("/lib64/ld-linux-x86-64.so.2");
        for length in 0..bytes.len() {
            assert_eq!(elf_libc(&bytes[..length]), None);
        }
        for (offset, value) in [(4, 1), (5, 2), (56, 0), (64, 1), (72, 255)] {
            let mut invalid = bytes.clone();
            invalid[offset] = value;
            assert_eq!(elf_libc(&invalid), None);
        }
        assert_eq!(elf_libc(&executable("ld-linux-x86-64.so.2")), None);
        assert_eq!(elf_libc(&executable("/lib/unknown-loader")), None);
    }
}
