use anyhow::{bail, Context, Result};

pub(crate) fn validate_requested(version: &str) -> Result<()> {
    let patch = version
        .strip_prefix("1.14.")
        .context("only stable 1.14.x runtimes are supported")?;
    if patch.is_empty()
        || !patch.bytes().all(|byte| byte.is_ascii_digit())
        || (patch.len() > 1 && patch.starts_with('0'))
    {
        bail!("invalid stable 1.14.x runtime version");
    }
    patch
        .parse::<u32>()
        .context("invalid runtime patch version")?;
    Ok(())
}

pub(crate) fn validate_output(output: &str, expected: &str) -> Result<()> {
    let actual = output
        .lines()
        .find_map(|line| line.strip_prefix("sing-box version "))
        .context("runtime version output has no version")?;
    validate_requested(actual)?;
    if actual != expected {
        bail!("runtime version does not match requested version");
    }
    let tags = output
        .lines()
        .find_map(|line| line.strip_prefix("Tags: "))
        .context("runtime version output has no build tags")?;
    if !tags
        .split(',')
        .map(str::trim)
        .any(|tag| tag == "with_v2ray_api")
    {
        bail!("runtime requires the with_v2ray_api build tag");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_exact_stable_version_and_tag() {
        let valid = "sing-box version 1.14.2\nEnvironment: go1.26.1 linux/amd64\nTags: with_quic,with_v2ray_api\n";
        assert!(validate_output(valid, "1.14.2").is_ok());
        for output in [
            valid.replace("1.14.2", "1.13.2"),
            valid.replace("1.14.2", "1.14.3"),
            valid.replace("1.14.2", "1.14.2-beta.1"),
            valid.replace("with_v2ray_api", "not_with_v2ray_api"),
            valid.replace("Tags:", "Features:"),
        ] {
            assert!(validate_output(&output, "1.14.2").is_err());
        }
        for version in ["1.14.", "1.14.02", "v1.14.2", "1.14.2+dirty", "1.15.0"] {
            assert!(validate_requested(version).is_err());
        }
    }
}
