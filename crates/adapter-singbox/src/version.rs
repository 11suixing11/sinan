use anyhow::{Context, Result, bail};

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

pub(crate) fn validate_features(output: &str, required: Option<&str>) -> Result<()> {
    let Some(required) = required else {
        return Ok(());
    };
    anyhow::ensure!(
        required.len() <= 4096,
        "runtime feature requirements exceed size limit"
    );
    let required: Vec<String> =
        serde_json::from_str(required).context("invalid runtime feature requirements")?;
    anyhow::ensure!(
        required.len() <= 32
            && required.iter().all(|feature| matches!(
                feature.as_str(),
                "with_utls"
                    | "with_quic"
                    | "with_naive_outbound"
                    | "with_clash_api"
                    | "with_v2ray_api"
            )),
        "unknown runtime feature requirement"
    );
    let tags = output
        .lines()
        .find_map(|line| line.strip_prefix("Tags: "))
        .context("runtime version output has no build tags")?;
    for feature in required {
        if !tags.split(',').map(str::trim).any(|tag| tag == feature) {
            bail!("runtime lacks a required build feature for the candidate configuration");
        }
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

    #[test]
    fn candidate_features_must_match_exact_compiled_tags() {
        let output =
            "sing-box version 1.14.2\nTags: with_v2ray_api,with_clash_api,with_utls,with_quic\n";
        assert!(validate_features(output, None).is_ok());
        assert!(validate_features(output, Some(r#"["with_clash_api","with_quic"]"#)).is_ok());
        for requirement in [
            r#"["with_naive_outbound"]"#,
            r#"["clash_api"]"#,
            r#"["secret"]"#,
            r#"{"with_clash_api":true}"#,
        ] {
            assert!(validate_features(output, Some(requirement)).is_err());
        }
        assert!(
            validate_features(
                &output.replace("with_clash_api", "not_with_clash_api"),
                Some(r#"["with_clash_api"]"#)
            )
            .is_err()
        );
    }
}
