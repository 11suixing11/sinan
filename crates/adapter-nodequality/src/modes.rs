use super::*;

pub(super) struct Mode {
    pub name: &'static str,
    pub targets: Option<String>,
}

pub(super) fn validate(spec: &DiagnosticSpec) -> Result<Mode> {
    let mode = spec
        .options
        .get("mode")
        .map(String::as_str)
        .unwrap_or("full");
    if !supports_modes(&spec.version)
        && spec.options.keys().any(|key| {
            matches!(
                key.as_str(),
                "mode" | "daily_targets" | "environment_section"
            )
        })
    {
        bail!("legacy diagnostic artifacts do not support mode options");
    }
    if spec
        .options
        .get("environment_section")
        .is_some_and(|value| value != "true")
    {
        bail!("invalid diagnostic environment option");
    }
    if mode == "full" {
        if spec.options.contains_key("daily_targets") {
            bail!("full diagnostics cannot accept daily targets");
        }
        return Ok(Mode {
            name: "full",
            targets: None,
        });
    }
    if mode != "daily"
        || spec
            .options
            .get("network_mode")
            .is_some_and(|value| value != "low")
        || spec
            .options
            .get("upload_report")
            .is_some_and(|value| value != "false")
    {
        bail!("daily diagnostics require bounded network mode without public upload");
    }
    let targets = spec
        .options
        .get("daily_targets")
        .context("daily targets missing")?;
    if targets.len() > 8192 {
        bail!("daily targets exceed byte limit");
    }
    let value: serde_json::Value = serde_json::from_str(targets)?;
    let entries = value.as_array().context("daily targets must be an array")?;
    if entries.len() > 4 {
        bail!("daily diagnostics accept at most four targets");
    }
    for entry in entries {
        let item = entry
            .as_object()
            .context("daily target must be an object")?;
        let name = entry["name"]
            .as_str()
            .context("daily target name missing")?;
        let target = entry["target"]
            .as_str()
            .context("daily target address missing")?;
        let port = entry["port"]
            .as_u64()
            .context("daily target port missing")?;
        if item.len() != 3
            || name.trim().is_empty()
            || name.len() > 128
            || name.chars().any(char::is_control)
            || target.is_empty()
            || target.len() > 253
            || target.starts_with('-')
            || !target
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b".:-".contains(&byte))
            || !(1..=65535).contains(&port)
        {
            bail!("invalid daily TCP target");
        }
    }
    Ok(Mode {
        name: "daily",
        targets: Some(targets.clone()),
    })
}
